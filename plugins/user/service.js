'use strict'

const { saltHashPassword, checksaltHashPassword } = require('../../util/salthash');
class UserService {
    constructor(userCollection, globalCollection) {
        this.userCollection = userCollection;
        this.globalCollection = globalCollection;
    }

    async register(username, email, password) {
        let dbResult;
        try {
            const _username = username.replace(/ /g, '').trim();
            const _email = email.replace(/ /g, '').trim();
            // const name = await this.globalCollection.find({ username: _username }).toArray();
            // const mail = await this.globalCollection.find({ email: _email }).toArray();
            // if (name.length) throw new Error('username taken');
            // if (mail.length) throw new Error('email taken');
            const saltedpassword = await saltHashPassword(password);
            const { value } = await this.globalCollection.findOneAndUpdate({ nextUid: { '$exists': 1 } }, { $inc: { nextUid: 1 } }, { returnOriginal: true, upsert: true });
            const uid = parseInt(value.nextUid, 10);
            if (!uid) throw new Error('Can not get uid from database');
            await Promise.all([
                await this.globalCollection.insertOne({ username: _username }),
                await this.globalCollection.insertOne({ email: _email }),
                dbResult = await this.userCollection.insertOne({ uid: uid, username: _username, email: _email, saltedpassword: saltedpassword }),
            ]);
        } catch (e) {
            throw e;
        }
        return dbResult.insertedId;
    }

    async login(username, password) {
        const _username = username.replace(/ /g, '').trim();
        const users = await this.userCollection.find({ username: _username }).toArray()
        const { uid } = users[0]
        const checkSalt = await checksaltHashPassword(users[0].saltedpassword, password);
        if (!uid || !checkSalt) throw new Error('Failed to login')
        return { uid }
    }

    updateProfile(uid, updateType) {
        const _uid = parseInt(uid, 10);
        const { type, path } = updateType;
        if (type === 'avatar') {
            return this.userCollection.findOneAndUpdate({ uid: _uid }, { $set: { avatar: path } }, { upsert: true, projection: { uid: 1 } });
        }
    }


    getProfile(uid) {
        return this.userCollection.findOne({ uid }, { projection: { _id: 0, saltedpassword: 0 } })
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