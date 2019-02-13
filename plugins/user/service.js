'use strict'

const { saltHashPassword, checksaltHashPassword } = require('../../util/salthash');
let registerIsRunning = false;
class UserService {
    constructor(userCollection, globalCollection) {
        this.userCollection = userCollection
        this.globalCollection = globalCollection
    }
    async register(username, email, password) {
        if (registerIsRunning === true) {
            throw new Error('Register is too busy please try again later');
        }
        registerIsRunning = true;
        try {
            const name = await this.globalCollection.find({ username: username }).toArray()
            const mail = await this.globalCollection.find({ email: email }).toArray();
            if (name.length) throw new Error('username taken');
            if (mail.length) throw new Error('email taken');
        } catch (e) {
            registerIsRunning = false;
            throw e
        }
        const saltedpassword = saltHashPassword(password);
        let dbResult
        try {
            const { value } = await this.globalCollection.findOneAndUpdate({ nextUid: { '$exists': 1 } }, { $inc: { nextUid: 1 } }, { returnOriginal: true, upsert: true })
            const uid = parseInt(value.nextUid, 10)
            if (!uid) throw new Error('Can not get uid from database');
            await this.globalCollection.insertOne({ username: username });
            await this.globalCollection.insertOne({ email: email });
            dbResult = await this.userCollection.insertOne({ uid, username, email, saltedpassword })
        } catch (e) {
            registerIsRunning = false;
            throw e
        }
        registerIsRunning = false;
        return dbResult.insertedId
    }

    async login(username, password) {
        const users = await this.userCollection.find({ username }).toArray()
        const { _id } = users[0]
        const checkSalt = checksaltHashPassword(users[0].saltedpassword, password);
        if (!_id || !checkSalt) throw new Error('Failed to login')
        return { _id }
    }

    getProfile(_id) {
        return this.userCollection.findOne({ _id }, { projection: { saltedpassword: 0 } })
    }

    async ensureIndexes(db) {
        await db.command({
            'collMod': this.userCollection.collectionName,
            validator: {
                uid: { $type: 'number' },
                username: { $type: 'string' },
                email: { $type: 'string' },
                saltedpassword: { $type: 'string' },
            }
        })
        await this.userCollection.createIndex({ uid: 1 }, { unique: true })
    }
}

module.exports = UserService