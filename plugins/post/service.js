'use strict'

function convertUserIdToStringInPost(t) {
    t.user._id = t.user._id.toString('hex')
    return t
}

class PostService {
    constructor(postCollection) {
        this.postCollection = postCollection
    }

    async fetchposts(userIds) {
        const posts = await this.postCollection.find({
            'user._id': { $in: userIds }
        }).sort({ createdAt: -1 }).toArray()
        return posts.map(convertUserIdToStringInPost)
    }

    async addPost(user, text) {
        await this.postCollection.insertOne({
            user,
            text,
            createdAt: new Date()
        })
    }

    async ensureIndexes(db) {
        await db.command({
            'collMod': this.postCollection.collectionName,
            validator: {
                user: { $type: 'object' },
                'user._id': { $type: 'string' },
                //'user.username': { $type: 'string' },
                text: { $type: 'string' }
            }
        })
        await this.postCollection.createIndex({ 'user._id': 1 })
    }
}

module.exports = PostService