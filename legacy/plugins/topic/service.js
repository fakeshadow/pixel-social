'use strict'

class TopicService {
    constructor(topicCollection, globalCollection) {
        this.topicCollection = topicCollection;
        this.globalCollection = globalCollection;
    }

    async getTopics(cid, lastPostTime) {
        try {
            const _lastPostTime = new Date(lastPostTime);
            _lastPostTime.toISOString();

            return this.topicCollection.aggregate([
                { $match: { $and: [{ cid: cid }, { lastPostTime: { $lt: _lastPostTime } }] } },
                { $sort: { lastPostTime: -1 } },
                { $limit: 20 },
                { $project: { _id: 0 } },
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
                { $unwind: "$user" }
            ]).toArray();
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