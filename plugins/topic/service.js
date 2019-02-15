'use strict'

let topicIsRunning = false;

class TopicService {
    constructor(topicCollection, globalCollection) {
        this.topicCollection = topicCollection;
        this.globalCollection = globalCollection;
    }
    async getTopic(tid) {
        try {

        } catch (e) {
            throw e;
        }
    }

    async getTopics(cid) {
        try {
            const _cid = parseInt(cid, 10)
            const array = await this.globalCollection.find({ cid: _cid }).sort({ tid: -1 }).toArray();
            const { tids } = array[0];
            return tids;
        } catch (e) {
            throw e
        }
    }

    async addTopic(uid, cid, pid, titleData) {
        if (topicIsRunning === true) {
            throw new Error('Topic is too busy please try again later');
        }
        topicIsRunning = true;
        try {
            const _cid = parseInt(cid, 10);
            const _uid = parseInt(uid, 10);
            const { value } = await this.globalCollection.findOneAndUpdate({ nextTid: { '$exists': 1 } }, { $inc: { nextTid: 1 } }, { returnOriginal: true, upsert: true });
            const tid = parseInt(value.nextTid, 10);
            if (!tid) throw new Error('Can not get tid from database');
            await Promise.all([
                this.topicCollection.insertOne({ uid: _uid, tid: tid, mainPid: pid, titleData: titleData, createdAt: new Date() }),
                this.globalCollection.findOneAndUpdate({ cid: _cid }, { $push: { tids: tid } }, { upsert: true })
            ])
            topicIsRunning = false;
            return tid;
        } catch (e) {
            topicIsRunning = false;
            throw e
        }
    }

    async ensureIndexes(db) {
        await db.command({
            'collMod': this.topicCollection.collectionName,
            validator: {
                uid: { $type: 'number' },
                tid: { $type: 'number' },
                mainPid: { $type: 'number' },
                titleData: { $type: 'string' },
            }
        })
        await this.topicCollection.createIndex({ 'tid': 1 }, { unique: true })
    }
}

module.exports = TopicService