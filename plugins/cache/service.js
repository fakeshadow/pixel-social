'use strict'

const { postStringify, postsStringify, topicStringify, topicsStringify, userStringify } = require('../../util/fastStringify');
const { mapUid, alterTopics, parseCache, alterPosts } = require('../../util/sortIds');

class CacheService {
    constructor(redis) {
        this.redis = redis;
    }

    async deleteCache() {
        await this.redis.flushall();
    }

    async getUserCache(request) {
        const { uid } = request;
        const _uid = parseInt(uid, 10);
        if (!_uid) return;
        const cachedUser = await this.redis.zrangebyscore('users', _uid, _uid);
        if (cachedUser.length) {
            const user = await parseCache(cachedUser);
            return userStringify(user[0]);
        }
        return null;
    }

    async getPostsCache(request) {
        const { toTid, lastPid } = request;
        const _lastPid = parseInt(lastPid, 10);
        const _toTid = parseInt(toTid, 10);
        if (_lastPid < 0) throw new Error('wrong page');

        const postsCacheAll = await this.redis.zrangebyscore(`toTid:${_toTid}`, `(${_lastPid}`, '+inf');
        const postsCache = postsCacheAll.slice(0, 20)
        if (postsCache.length > 0) {
            const posts = await parseCache(postsCache);
            const uidsMap = await mapUid(posts);
            const usersCache = await mapUsersCache(uidsMap, this.redis);
            const users = await parseCache(usersCache);
            const postsFinal = await alterPosts(posts, users);
            return postsStringify(postsFinal);
        }
        return null;
    }

    async getTopicsCache(request) {
        const { cids, lastPostTime } = request;
        const _cid = cids[0];
        const timeString = new Date(lastPostTime).toISOString();
        const timeScore = Date.parse(timeString);

        const topicsCacheAll = await this.redis.zrevrangebyscore(`topics:${_cid}`, `(${timeScore}`, '-inf');
        const topicsCache = topicsCacheAll.slice(0, 20)
        if (topicsCache.length > 1) {
            const topics = await parseCache(topicsCache);
            const uidsMap = await mapUid(topics);
            const usersCache = await mapUsersCache(uidsMap, this.redis);
            const users = await parseCache(usersCache);
            const topicsFinal = await alterTopics(topics, users);
            return topicsStringify(topicsFinal);
        }
        return null;
    }

    addUsersCache(payload) {
        return new Promise(resolve => {
            payload.forEach(async data => {
                const { user } = data;
                const { uid } = user;
                await this.redis.zremrangebyscore('users', uid, uid)
                await this.redis.zadd('users', uid, userStringify(user));
            })
            return resolve();
        })

    }

    addPostsCache(payload) {
        return new Promise(resolve => {
            if (!payload.length) return resolve(payload);

            payload.forEach(async postData => {
                this.refreshPostCache(postData).catch(e => reject(e))
            })
            return resolve();
        })
    }

    addTopicsCache(payload) {
        return new Promise((resolve, reject) => {
            if (!payload.length) return resolve(payload);

            payload.forEach(async topicData => {
                this.refreshTopicCache(topicData).catch(e => reject(e))
            })
            return resolve();
        })
    }

    async refreshUserCache(payload) {
        const { uid } = payload;
        await this.redis.zremrangebyscore('users', uid, uid);
        this.redis.zadd('users', uid, userStringify(payload));
        return payload;
    }

    async refreshPostCache(postData) {
        const { toTid, pid } = postData
        await this.redis.zremrangebyscore(`toTid:${toTid}`, pid, pid)
        await this.redis.zadd(`toTid:${toTid}`, pid, postStringify(postData))
    }

    async refreshTopicCache(topicData) {
        try {
            const { cid, tid, lastPostTime } = topicData;
            const timeString = new Date(lastPostTime).toISOString();
            const timeScoreNew = Date.parse(timeString);
            const timeScoreOld = await this.redis.zrangebyscore('topics:time', tid, tid);
            if (timeScoreOld.length) {
                await Promise.all([
                    await this.redis.zremrangebyscore(`topics:${cid}`, timeScoreOld[0], timeScoreOld[0]),
                    await this.redis.zremrangebyscore('topics:time', tid, tid),
                ])
            }
            await Promise.all([
                await this.redis.zadd(`topics:${cid}`, timeScoreNew, topicStringify(topicData)),
                await this.redis.zadd('topics:time', tid, timeScoreNew)
            ])
        } catch (e) {
            throw e
        }
    }
}

module.exports = CacheService;

const mapUsersCache = (uidsMap, redis) => {
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