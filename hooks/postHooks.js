'use strict'

const { mapUid, alterPosts, parseCache } = require('../util/sortIds');
const { postStringify, postsStringify, topicStringify, userStringify } = require('../util/fastStringify');


async function postPreHook(req, res) {
    try {
        //    await this.redis.flushall();
        const { toTid, lastPid } = req.body;
        const _lastPid = parseInt(lastPid, 10);
        const _toTid = parseInt(toTid, 10);
        if (_lastPid < 0) throw new Error('wrong page');

        const postsCache = await this.redis.zrange(`toTid:${_toTid}`, _lastPid, _lastPid + 19);

        if (postsCache.length) {
            const posts = await parseCache(postsCache);
            const uidsMap = await mapUid(posts);
            const usersCache = await getUsersCache(uidsMap, this.redis);
            const users = await parseCache(usersCache);
            const postsFinal = await alterPosts(posts, users);
            const string = postsStringify(postsFinal);
            return res.send(string);
        }
    } catch (err) {
        res.send(err)
    }
}

async function postPreSerialHook(req, res, payload) {
    try {
        if (Array.isArray(payload)) {
            if (!payload.length) return payload;
            const { toTid } = payload[0]

            payload.forEach(async post => {
                const { pid } = post;
                await this.redis.zremrangebyscore(`toTid:${toTid}`, pid, pid)
                this.redis.zadd(`toTid:${toTid}`, pid, postStringify(post))
            })

            addUsersCache(payload, this.redis);
            return payload;
        }

        // const { toTid, pid, relatedPost, relatedTopic } = payload;

        // if (relatedPost !== null) {
        //     const { toTid, pid } = relatedPost
        //     await this.redis.zremrangebyscore(`toTid:${toTid}`, pid, pid)
        //     this.redis.zadd(`toTid:${toTid}`, pid, postStringify(relatedPost))
        // }

        // if (relatedTopic !== null) {
        //     this.cacheService.refreshTopicCache(relatedTopic);
        // }

        // await this.redis.zremrangebyscore(`toTid:${toTid}`, pid, pid)
        // this.redis.zadd(`toTid:${toTid}`, pid, postStringify(payload))

        // return { message: 'success' };
    } catch (err) {
        res.send(err)
    }
}

module.exports = {
    postPreHook,
    postPreSerialHook
}

// update users detialed info set with payload.databse
const addUsersCache = (payload, redis) => {
    payload.forEach(async data => {
        const { user } = data;
        const { uid } = user;
        const _uid = await redis.zrangebyscore('users', uid, uid)
        if (!_uid.length) {
            redis.zadd('users', uid, userStringify(user));
        }
    })
}

const getUsersCache = (uidsMap, redis) => {
    return new Promise(resolve => {
        let usersCache = [];
        uidsMap.forEach(async (uid, index) => {
            const user = await redis.zrangebyscore('users', uid, uid);
            usersCache = usersCache.concat(user);
            if (index === uidsMap.length - 1) {
                return resolve(usersCache);
            }
        })
    })
}
