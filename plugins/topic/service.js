'use strict'
const { mapUid, alterTopics } = require('../../util/sortIds');

class TopicService {
    constructor(topicCollection, userCollection, globalCollection) {
        this.topicCollection = topicCollection;
        this.userCollection = userCollection;
        this.globalCollection = globalCollection;
    }

    async getTopics(cids, page) {
        try {
            // check if catergories are legit
            const _cids = await this.globalCollection.find({ cid: { $in: cids } }).toArray();
            if (_cids.length !== cids.length) throw new Error('Wrong Categories');

            // find topics by page. need to introduce last reply time
            const _page = parseInt(page, 10);
            const array = await this.topicCollection.find({ cid: { $in: cids } }, { projection: { _id: 0 } }).sort({ lastPostTime: -1 }).toArray();

            // each page have 50 topics
            const start = (_page - 1) * 50
            if (start < 0 || start > array.length) {
                return [];
            }

            // map topics and get all the userId
            const topicsMap = array.slice(start, start + 50);
            const uidsMap = await mapUid(topicsMap);

            // get all userId detail and map them to topics
            const uidsDetails = await this.userCollection.find({ uid: { $in: uidsMap }, }, { projection: { _id: 0, saltedpassword: 0 } }).toArray();
            const alteredTopics = await alterTopics(topicsMap, uidsDetails);

            // return raw result for building cache;
            return { 'cache': array, 'database': alteredTopics };
        } catch (e) {
            throw e
        }
    }

    async addTopic(uid, topicData) {
        try {
            const { value } = await this.globalCollection.findOneAndUpdate({ nextTid: { '$exists': 1 } }, { $inc: { nextTid: 1 } }, { returnOriginal: true, upsert: true });
            const _tid = parseInt(value.nextTid, 10);
            if (!_tid) throw new Error('Can not get tid from database');
            const _uid = parseInt(uid, 10);
            const _cid = topicData.cid;
            const _mainPid = parseInt(topicData.mainPid, 10);
            return await this.topicCollection.insertOne({ tid: _tid, cid: _cid, uid: _uid, mainPid: _mainPid, topicContent: topicData.topicContent, lastPostTime: new Date(), postCount: 0 });
        } catch (e) {
            throw e
        }
    }

    async ensureIndexes(db) {
        await db.command({
            'collMod': this.topicCollection.collectionName,
            validator: {
                tid: {
                    $type: 'number'
                },
                cid: {
                    $type: 'string'
                },
                uid: {
                    $type: 'number'
                },
                mainPid: {
                    $type: 'number'
                },
                topicContent: {
                    $type: 'string'
                },
                lastPostTime: {
                    $type: 'date'
                },
                postCount: {
                    $type: 'number'
                },
            }
        })
        await this.topicCollection.createIndex({ 'tid': 1 }, { unique: true })
    }
}

module.exports = TopicService