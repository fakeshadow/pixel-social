'use strict'

const DUPLICATE_KEY_ERROR_CODE = 11000;

class UserService {
    constructor(userCollection) {
        this.userCollection = userCollection
    }
    async register(username, email, password) {
        let dbResult
        try {
            dbResult = await this.userCollection.insertOne({ username, email, password })
        } catch (e) {
            if (e.code === DUPLICATE_KEY_ERROR_CODE) {
                throw new Error('Username Taken')
            }
            throw e
        }
        return dbResult.insertedId
    }
    async ensureIndexes(db) {
        await db.command({
            'collMod': this.userCollection.collectionName,
            validator: {
                username: { $type: 'string' },
                email: { $type: 'string' },
                password: { $type: 'string' },
            }
        })
        await this.userCollection.createIndex({ username: 1 }, { unique: true })
    }
}

module.exports = UserService