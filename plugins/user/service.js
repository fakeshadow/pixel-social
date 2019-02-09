'use strict'

const { saltHashPassword, checksaltHashPassword } = require('../../util/salthash');
const DUPLICATE_KEY_ERROR_CODE = 11000;

class UserService {
    constructor(userCollection) {
        this.userCollection = userCollection
    }
    async register(username, email, password) {
        const saltedpassword = saltHashPassword(password);
        let dbResult
        try {
            dbResult = await this.userCollection.insertOne({ username, email, saltedpassword })
        } catch (e) {
            if (e.code === DUPLICATE_KEY_ERROR_CODE) {
                throw new Error('Username Taken')
            }
            throw e
        }
        return dbResult.insertedId
    }

    async login(username, password) {
        const users = await this.userCollection.find({ username }, { projection: { password: 0 } }).toArray()
        const { _id } = users[0]
        const checkSalt = checksaltHashPassword(users[0].saltedpassword, password);
        if (!_id || !checkSalt) throw new Error('Failed to login')
        return { _id }
    }

    getProfile(_id) {
        return this.userCollection.findOne({ _id }, { projection: { password: 0 } })
    }

    async ensureIndexes(db) {
        await db.command({
            'collMod': this.userCollection.collectionName,
            validator: {
                username: { $type: 'string' },
                email: { $type: 'string' },
                saltedpassword: { $type: 'string' },
            }
        })
        await this.userCollection.createIndex({ username: 1 }, { unique: true })
    }
}

module.exports = UserService