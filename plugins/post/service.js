'use strict'
const { mapUid, alterPosts } = require('../../util/sortIds');

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
                    array = await this.postCollection.find({ toTid: _toTid }, { projection: { _id: 0 } }).sort({ pid: 1 }).toArray(),
                    firstPost = await this.postCollection.findOne({ pid: mainPid }, { projection: { _id: 0 } })
                ]);
                array.splice(0, 0, firstPost)
            } else if (_toTid > 0 && _toPid === 0 && _page > 1) {
                array = await this.postCollection.find({ toTid: _toTid }, { projection: { _id: 0 } }).sort({ pid: 1 }).toArray();

                // get reply posts for an post.  
            } else if (_toTid === 0 && _toPid > 0) {
                array = await this.postCollection.find({ toPid: _toPid }, { projection: { _id: 0 } }).sort({ pid: 1 }).toArray();
            } else {
                throw new Error('wrong tid, pid or page');
            }

            // each page have 50 posts
            const start = (_page - 1) * 50
            if (start < 0 || start >= array.length) {
                return { 'cache': [], 'database': [] };
            }

            const cacheArray = [...array];
            const mappedArray = array.slice(start, start + 50);

            // get uid details and map them to posts
            const uidsMap = await mapUid(mappedArray);
            const uidsDetails = await this.userCollection.find({ uid: { $in: uidsMap }, }, { projection: { _id: 0, saltedpassword: 0, email: 0 } }).toArray();
            const alteredPosts = await alterPosts(mappedArray, uidsDetails);

            // return both origin and altered array and past the former one to redis hook for caching
            return { 'cache': cacheArray, 'database': alteredPosts }
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
            const { value } = await this.globalCollection.findOneAndUpdate({ nextPid: { '$exists': 1 } }, { $inc: { nextPid: 1 } }, { returnOriginal: true, upsert: true })
            const pid = parseInt(value.nextPid, 10)
            if (!pid) throw new Error('Can not get pid from database')

            // add a new topic if topicData is not null
            let _tid = 0;
            let _cid, _topicContent;
            if (topicData !== null && _toPid === 0 && _toTid === 0) {
                const { value } = await this.globalCollection.findOneAndUpdate({ nextTid: { '$exists': 1 } }, { $inc: { nextTid: 1 } }, { returnOriginal: true, upsert: true });
                if (!value.nextTid) throw new Error('Can not get tid from database');
                _tid = value.nextTid;
                _cid = topicData.cid;
                _topicContent = topicData.topicContent;
            }

            const date = new Date();
            await Promise.all([
                await this.postCollection.insertOne({ uid: _uid, pid: pid, toTid: _toTid, toPid: _toPid, postContent: postContent, postCount: 0, createdAt: date }),
                _toPid > 0 ? await this.postCollection.findOneAndUpdate({ pid: _toPid }, { $inc: { postCount: 1 } }, { upsert: true }) : null,
                _toTid > 0 ? await this.topicCollection.findOneAndUpdate({ tid: _toTid }, { $set: { lastPostTime: date }, $inc: { postCount: 1 } }, { upsert: true }) : null,
                _tid > 0 ? await this.topicCollection.insertOne({ tid: _tid, cid: _cid, uid: _uid, mainPid: pid, topicContent: _topicContent, lastPostTime: date, postCount: 0 }) : null,
            ]);

            const rawPostNew = {
                uid: _uid,
                pid: pid,
                toTid: _toTid,
                toPid: _toPid,
                postContent: postContent,
                postCount: 0,
                createdAt: date,
                isTopicMain: _tid > 0 ? _tid : null,
                // below is topic schema
                rawTopicNew: _tid > 0 ? {
                    tid: _tid,
                    cid: _cid,
                    mainPid: pid,
                    topicContent: _topicContent,
                    postCount: 0,
                    lastPostTime: date,
                } : null
            }
            // return raw post for updating cache
            return rawPostNew;
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
            // return old value to popluate raw post object
            const { value } = await this.postCollection.findOneAndUpdate({ uid: _uid, pid: _pid, }, { $set: { postContent: postContent, createdAt: date } }, { returnOriginal: true, upsert: true });
            const { toTid, toPid, postCount } = value;

            let isTopicMain = 0;
            if (toPid === 0 && toTid === 0) {
                const { tid } = await this.topicCollection.findOne({ mainPid: _pid }, { projection: { _id: 0, tid: 1 } });
                isTopicMain = tid
            }

            const rawPostNew = {
                uid: _uid,
                pid: _pid,
                toTid: toTid,
                toPid: toPid,
                postContent: postContent,
                postCount: postCount,
                createdAt: date,
                isTopicMain: isTopicMain
            }
            // return raw post for updating cache
            return rawPostNew;
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