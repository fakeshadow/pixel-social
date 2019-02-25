'use strict'

const { parseCache } = require('../util/sortIds');
const { userStringify } = require('../util/fastStringify');

async function userPreHook(req, res) {
    try {
        const { uid } = req.body;
        const _uid = parseInt(uid, 10);
        if (!_uid) return;
        const cachedUser = await this.redis.zrangebyscore('users', _uid, _uid);
        if (cachedUser.length) {
            const user = await parseCache(cachedUser);
            const userCache = await userStringify(user[0]);
            return res.send(userCache);
        }
    } catch (err) {
        res.send(err)
    }
}

async function userPreSerialHook(req, res, payload) {
    try {
        const { uid } = payload;
        await this.redis.zremrangebyscore('users', uid, uid);
        this.redis.zadd('users', uid, userStringify(payload));
        return payload;
    } catch (err) {
        res.send(err)
    }
}

module.exports = {
    userPreHook,
    userPreSerialHook
}
