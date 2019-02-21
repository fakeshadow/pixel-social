'use strict'
const { mapUid, alterArray } = require('../../util/sortIds');

class PostService {
    constructor(topicCollection, postCollection, userCollection, globalCollection) {
        this.topicCollection = topicCollection;
        this.postCollection = postCollection;
        this.userCollection = userCollection
        this.globalCollection = globalCollection;
    }

    async getPosts(uid, requestBody) {
        try {
            const selfUid = parseInt(uid, 10);
            const otherUid = parseInt(requestBody.uid, 10);
            const _page = parseInt(requestBody.page, 10);
            const _toTid = parseInt(requestBody.toTid, 10);
            const _toPid = parseInt(requestBody.toPid, 10);

            let array;

            // get posts for an topic add topic's self post into array if the page is 1
            if (_toTid > 0 && _toPid === 0 && _page === 1) {
                let firstPost;
                const { mainPid } = await this.topicCollection.findOne({ tid: _toTid }, { projection: { mainPid: 1 } });
                await Promise.all([
                    array = await this.postCollection.find({ toTid: _toTid }).sort({ pid: 1 }).toArray(),
                    firstPost = await this.postCollection.findOne({ pid: mainPid })
                ]);
                array.splice(0, 0, firstPost);
            } else if (_toTid > 0 && _toPid === 0 && _page > 1) {
                array = await this.postCollection.find({ toTid: _toTid }).sort({ pid: 1 }).toArray();

                // get reply posts for an post.  
            } else if (_toTid === 0 && _toPid > 0) {
                array = await this.postCollection.find({ toPid: _toPid }).sort({ pid: 1 }).toArray();
            } else {
                throw new Error('wrong tid, pid or page');
            }

            // each page have 50 posts
            const start = (_page - 1) * 50
            if (start < 0 || start >= array.length) {
                return [];
            }
            const arrayMap = array.slice(start, start + 50);

            // get uid details and map them to posts
            const uidsMap = await mapUid(arrayMap);
            const uidsDetails = await this.userCollection.find({ uid: { $in: uidsMap }, }, { projection: { _id: 0, saltedpassword: 0, email: 0 } }).toArray();
            return alterArray(arrayMap, uidsDetails);
        } catch (e) {
            throw (e)
        }
    }

    async addPost(uid, postData) {
        try {
            const _uid = parseInt(uid, 10);
            const _toPid = parseInt(postData.toPid, 10);
            const _toTid = parseInt(postData.toTid, 10);
            const { value } = await this.globalCollection.findOneAndUpdate({ nextPid: { '$exists': 1 } }, { $inc: { nextPid: 1 } }, { returnOriginal: true, upsert: true })
            const pid = parseInt(value.nextPid, 10)
            if (!pid) throw new Error('Can not get pid from database')

            await Promise.all([
                await this.postCollection.insertOne({ uid: _uid, pid: pid, toTid: _toTid, toPid: _toPid, postContent: postData.postContent, postCount: 0, createdAt: new Date() }),
                _toPid > 0 ? await this.postCollection.findOneAndUpdate({ pid: _toPid }, { $inc: { postCount: 1 } }, { upsert: true }) : null,
                _toTid > 0 ? await this.topicCollection.findOneAndUpdate({ tid: _toTid }, { $set: { lastPostTime: new Date() }, $inc: { postCount: 1 } }, { upsert: true }) : null,
            ]);
            return pid;
        } catch (e) {
            throw e
        }
    }

    async editPost(uid, postData) {
        try {
            const _uid = parseInt(uid, 10);
            const _pid = parseInt(postData.pid, 10);
            await this.postCollection.findOneAndUpdate({ uid: _uid, pid: _pid, }, { $set: { postContent: postData.postContent, createdAt: new Date() } }, { upsert: true })
        } catch (e) {
            throw e;
        }
    }

    async ensureIndexes(db) {
        await db.command({
            'collMod': this.postCollection.collectionName,
            validator: {
                uid: { $type: 'number' },
                pid: { $type: 'number' },
                toTid: { $type: 'number' },
                toPid: { $type: 'number' },
                postContent: { $type: 'string' },
                postCount: { $type: 'number' },
                createdAt: { $type: 'date' }
            }
        })
        await this.postCollection.createIndex({ 'pid': 1 }, { unique: true })
    }
}

module.exports = PostService