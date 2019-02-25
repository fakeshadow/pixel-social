'use strict'

class TopicService {
    constructor(topicCollection, globalCollection) {
        this.topicCollection = topicCollection;
        this.globalCollection = globalCollection;
    }

    async getTopics(cids, page) {
        try {
            const _cids = await this.globalCollection.find({ cid: { $in: cids } }).toArray();
            if (_cids.length !== cids.length) throw new Error('Wrong Categories');

            return this.topicCollection.aggregate(
                { $match: { cid: { $in: cids } } },
                { $project: { _id: 0 } },
                { $sort: { lastPostTime: -1 } },
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


            // const _page = parseInt(page, 10);
            // const start = (_page - 1) * 50
            // if (start < 0 || start > array.length) {
            //     return { 'cache': [], 'database': [] };
            // }

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