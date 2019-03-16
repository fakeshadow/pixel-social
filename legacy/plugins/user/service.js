'use strict'

const { saltHashPassword, checksaltHashPassword } = require('../../util/salthash');

class UserService {
    constructor(userCollection, globalCollection) {
        this.userCollection = userCollection;
        this.globalCollection = globalCollection
    }

    async register(username, email, password) {
        try {
            const _username = username.replace(/ /g, '').trim();
            const _email = email.replace(/ /g, '').trim();
            const result = await this.userCollection.find({ $or: [{ username: _username }, { email: _email }] }).toArray();
            if (result.length) throw new Error('username or email taken');

            const saltedpassword = await saltHashPassword(password);
            const { value } = await this.globalCollection.findOneAndUpdate({ nextUid: { $gt: 0 } }, { $inc: { nextUid: 1 } }, { returnOriginal: true, upsert: true });
            if (!value) throw new Error('Can not get uid from database');
            const { nextUid } = value;

            return await this.userCollection.insertOne({ uid: nextUid, username: _username, email: _email, saltedpassword: saltedpassword, avatar: '' });
        } catch (e) {
            throw e;
        }
    }

    async login(user, pass) {
        const _username = user.replace(/ /g, '').trim();
        const { saltedpassword, username, uid, email, avatar } = await this.userCollection.findOne({ username: _username }, { projection: { _id: 0 } });
        const checkSalt = await checksaltHashPassword(saltedpassword, pass);
        if (!uid || !checkSalt) throw new Error('Failed to login')
        return { uid, username, email, avatar }
    }

    async updateProfile(uid, userData) {
        const _uid = parseInt(uid, 10);
        const { avatar } = userData;
        const { value } = await this.userCollection.findOneAndUpdate({ uid: _uid }, { $set: { avatar: avatar } }, { returnOriginal: false, upsert: true, projection: { _id: 0, saltedpassword: 0 } });
        return value;
    }

    async getProfile(uid) {
        const _uid = parseInt(uid, 10);
        if (!_uid) throw new Error('Wrong uid');
        const userData = await this.userCollection.findOne({ uid: _uid }, { projection: { _id: 0, saltedpassword: 0 } })
        if(!userData) throw new Error('No user found')
        return userData
    }

    async ensureIndexes(db) {
        await db.command({
            'collMod': this.userCollection.collectionName,
            validator: {
                uid: {
                    $type: 'number'
                },
                username: {
                    $type: 'string'
                },
                email: {
                    $type: 'string'
                },
                saltedpassword: {
                    $type: 'string'
                },
                avatar: {
                    $type: 'string'
                }
            }
        })
        await this.userCollection.createIndex({ uid: 1 }, { unique: true })
    }
}

module.exports = UserService