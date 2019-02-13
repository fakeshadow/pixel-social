'use strict'

let postIsRunning = false;

class PostService {
    constructor(postCollection, globalCollection) {
        this.postCollection = postCollection;
        this.globalCollection = globalCollection;
    }

    async getPosts(uid) {
        try {
            const _uid = parseInt(uid, 10)
            return this.postCollection.find({ uid: _uid }).sort({ createdAt: -1 }).toArray()
        } catch (e) {
            throw (e)
        }
    }

    async addPost(uid, toPid, postData) {
        if (postIsRunning === true) {
            throw new Error('Post is too busy please try again later');
        }
        postIsRunning = true;
        try {
            const _uid = parseInt(uid, 10);
            const _toPid = parseInt(toPid, 10);
            const { value } = await this.globalCollection.findOneAndUpdate({ nextPid: { '$exists': 1 } }, { $inc: { nextPid: 1 } }, { returnOriginal: true, upsert: true })
            const pid = parseInt(value.nextPid, 10)
            if (!pid) throw new Error('Can not get pid from database')
            await this.postCollection.insertOne({ uid: _uid, pid: pid, toPid: _toPid, postData: postData, createdAt: new Date() }) 
            postIsRunning = false;
            return pid;
        } catch (e) {
            postIsRunning = false;
            throw e
        }
    }

    async editPost(uid, pid, postData) {
        try {
            const _uid = parseInt(uid, 10);
            const _pid = parseInt(pid, 10);
            await this.postCollection.findOneAndUpdate({ uid: _uid, pid: _pid, }, { $set: { postData: postData, modifiedAt: new Date() } }, { upsert: true })
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
                toPid: { $type: 'number' },
                postData: { $type: 'string' },
            }
        })
        await this.postCollection.createIndex({ 'pid': 1 })
    }
}

module.exports = PostService