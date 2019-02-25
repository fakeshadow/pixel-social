'use strict'

class PostService {
    constructor(topicCollection, postCollection, globalCollection) {
        this.topicCollection = topicCollection;
        this.postCollection = postCollection;
        this.globalCollection = globalCollection;
    }

    async getPosts(uid, requestBody) {
        try {
            const { uid, toTid, toPid, lastPid } = requestBody
            const selfUid = parseInt(uid, 10);
            const _uid = parseInt(uid, 10);
            const _toPid = parseInt(toPid, 10);
            const _toTid = parseInt(toTid, 10);
            const _lastPid = parseInt(lastPid, 10);

            let query;
            if (_toTid > 0 && _toPid === 0) {
                query = { toTid: _toTid }
            } else if (_toTid === 0 && _toPid > 0) {
                query = { toPid: _toPid }
            } else {
                throw new Error('illegal query');
            }

            return await this.postCollection.aggregate(
                { $match: { $and: [query, { pid: { $gt: _lastPid } }] } },
                { $project: { _id: 0 } },
                { $sort: { pid: 1 } },
                { $limit: 20 },
                {
                    $lookup: {
                        from: 'users',
                        let: { uidDetail: '$uid' },
                        pipeline: [
                            { $match: { $expr: { $eq: ['$$uidDetail', '$uid'] } } },
                            { $project: { _id: 0, saltedpassword: 0 } }
                        ],
                        as: 'user'
                    }
                },
                { $unwind: "$user" },
                { $project: {} }).toArray();

        } catch (e) {
            throw (e)
        }
    }

    async addPost(uid, postData, topicData) {
        try {
            const { toPid, toTid, postContent } = postData;
            const _uid = parseInt(uid, 10);
            const _toPid = parseInt(toPid, 10);
            const _toTid = parseInt(toTid, 10);

            if (_toPid < 0 || _toTid < 0) throw new Error('illegal postData');
            if (topicData === null && _toTid === 0) throw new Error('illegal topicData');
            if (_toPid > 0 && _toTid > 0) {
                const toPidCheck = await this.postCollection.findOne({ toTid: _toTid, pid: _toPid });
                if (!toPidCheck) throw new Error('illegal reply request')
            }

            let _tid = 0;
            let _cid, _topicContent;
            if (topicData !== null && _toPid === 0 && _toTid === 0) {
                const { value } = await this.globalCollection.findOneAndUpdate({ nextTid: { '$exists': 1 } }, { $inc: { nextTid: 1 } }, { returnOriginal: true, upsert: true });
                if (!value.nextTid) throw new Error('Can not get tid from database');
                _tid = value.nextTid;
                _cid = topicData.cid;
                _topicContent = topicData.topicContent;
            }
            const { value } = await this.globalCollection.findOneAndUpdate({ nextPid: { '$exists': 1 } }, { $inc: { nextPid: 1 } }, { returnOriginal: true, upsert: true });
            const pid = value.nextPid;
            if (!pid) throw new Error('Can not get pid from database')

            const toTidFinal = _toTid === 0 ? _tid : _toTid

            const date = new Date();
            let relatedTopic, relatedPost, selfTopic;
            await Promise.all([
                await this.postCollection.insertOne({ uid: _uid, pid: pid, toTid: toTidFinal, toPid: _toPid, postContent: postContent, postCount: 0, createdAt: date }, { upsert: true }),
                relatedPost = _toPid > 0 ? await this.postCollection.findOneAndUpdate({ pid: _toPid }, { $inc: { postCount: 1 } }, { returnOriginal: false, upsert: true, projection: { _id: 0 } }) : null,
                relatedTopic = _toTid > 0 ? await this.topicCollection.findOneAndUpdate({ tid: _toTid }, { $set: { lastPostTime: date }, $inc: { postCount: 1 } }, { returnOriginal: false, upsert: true, projection: { _id: 0 } }) : null,
                selfTopic = _tid > 0 ? await this.topicCollection.insertOne({ tid: _tid, cid: _cid, uid: _uid, mainPid: pid, topicContent: _topicContent, lastPostTime: date, postCount: 0 }, { returnOriginal: false, upsert: true, projection: { _id: 0 } }) : null,
            ]);

            return {
                uid: _uid,
                pid: pid,
                toTid: toTidFinal,
                toPid: _toPid,
                postContent: postContent,
                postCount: 0,
                createdAt: date,
                selfTopic: selfTopic !== null ? selfTopic.value : null,
                relatedPost: relatedPost !== null ? relatedPost.value : null,
                relatedTopic: relatedTopic !== null ? relatedTopic.value : null,
            }
        } catch (e) {
            throw e
        }
    }

    async editPost(uid, postData) {
        try {
            const { pid, postContent } = postData
            const _uid = parseInt(uid, 10);
            const _pid = parseInt(pid, 10);
            const date = new Date();

            const { value } = await this.postCollection.findOneAndUpdate({ uid: _uid, pid: _pid, }, { $set: { postContent: postContent, createdAt: date } }, { returnOriginal: false, upsert: true, projection: { _id: 0 } });
            return value;
        } catch (e) {
            throw e;
        }
    }

    async ensureIndexes(db) {
        await db.command({
            'collMod': this.postCollection.collectionName,
            validator: {
                uid: {
                    $type: 'number'
                },
                pid: {
                    $type: 'number'
                },
                toTid: {
                    $type: 'number'
                },
                toPid: {
                    $type: 'number'
                },
                postContent: {
                    $type: 'string'
                },
                postCount: {
                    $type: 'number'
                },
                createdAt: {
                    $type: 'date'
                }
            }
        })
        await this.postCollection.createIndex({ 'pid': 1 }, { unique: true })
    }
}

module.exports = PostService