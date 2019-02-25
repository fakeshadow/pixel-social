'use strict'

const { postStringify, topicStringify, userStringify } = require('../util/fastStringify');
const { mapUid, alterTopics, parseCache } = require('../util/sortIds');
const arrayFlatten = require('../util/arrayflatten');

//check if the request content are in cache
async function topicPreHook(req, res) {
    try {
        //  await this.redis.flushall();

        // const { cids, page } = req.body;
        // const _page = parseInt(page, 10);
        // if (_page <= 0) throw new Error('wrong page');
        // const start = (_page - 1) * 50;

        // const promises = [];
        // cids.forEach(cid => promises.push(getTopicsCache(cid, start, this.redis)));
        // const topicsUnsort = await Promise.all(promises);
        // const isCached = isCidCached(topicsUnsort);
        // if (!isCached) return;

        // const topicsCache = arrayFlatten(topicsUnsort);
        // if (topicsCache.length) {
        //     const topics = await parseCache(topicsCache);
        //     const uidsMap = await mapUid(topics);
        //     const usersCache = await getUsersCache(uidsMap, this.redis);
        //     const users = await parseCache(usersCache);
        //     const topicsFinal = await alterTopics(topics, users);
        //     if (topicsFinal.length) {
        //         const string = topicsStringify({ 'cache': topicsFinal, 'database': [] })
        //         return res.send(string);
        //     }
        // }
    } catch (err) {
        res.send(err)
    }
}

async function topicPreSerialHook(req, res, payload) {
    try {
        // const { cache, database } = payload;
        // if (database !== undefined) {
        //     if (!database.length) return payload;

        //     cache.forEach(async topic => {
        //         const { cid, lastPostTime } = topic;
        //         const timeString = new Date(lastPostTime).toISOString();
        //         const timeScore = Date.parse(timeString);
        //         const cachedTopic = await this.redis.zrangebyscore(`topics:${cid}`, timeScore, timeScore);
        //         if (!cachedTopic.length) {
        //             const string = topicStringify(topic)
        //             await this.redis.zadd(`topics:${cid}`, timeScore, string);
        //         }
        //     })
        //     addUsersCache(database, this.redis);
        //     return { 'cache': [], 'database': database };

        // }
        // const _rawPostNew = payload;
        // const { uid, pid, rawTopicNew } = payload;
        // const { tid, cid, mainPid, topicContent, postCount, lastPostTime } = rawTopicNew;
        // const _rawTopicNew = {
        //     uid: uid,
        //     tid: tid,
        //     cid: cid,
        //     mainPid: mainPid,
        //     topicContent: topicContent,
        //     postCount: postCount,
        //     lastPostTime: lastPostTime,
        // }

        // const timeString = new Date(lastPostTime).toISOString();
        // const timeScore = Date.parse(timeString);
        // let cachedPost, cachedTopic;
        // await Promise.all([
        //     cachedTopic = await this.redis.zrangebyscore(`topics:${cid}`, timeScore, timeScore),
        //     cachedPost = await this.redis.zrangebyscore(`toTid:${tid}`, pid, pid)
        // ]);
        // if (!cachedTopic.length && !cachedPost.length) {
        //     this.redis.zadd(`toTid:${tid}`, pid, postStringify(_rawPostNew))
        //     this.redis.zadd(`topics:${cid}`, timeScore, topicStringify(_rawTopicNew))
        // } else if (cachedTopic.length && cachedPost.length) {
        //     await Promise.all([
        //         await this.redis.zremrangebyscore(`toTid:${tid}`, pid, pid),
        //         await this.redis.zremrangebyscore(`topics:${cid}`, timeScore, timeScore)
        //     ])
        //     this.redis.zadd(`toTid:${tid}`, pid, postStringify(_rawPostNew))
        //     this.redis.zadd(`topics:${cid}`, timeScore, topicStringify(_rawTopicNew))
        // } else {
        //     return 'conflict data in cache'
        // }

        // return 'success';
    } catch (err) {
        res.send(err)
    }
}

module.exports = {
    topicPreHook,
    topicPreSerialHook
}

// update users detialed info set with payload.databse
const addUsersCache = (database, redis) => {
    database.forEach(async data => {
        const { user } = data;
        const { uid } = user;
        const _uid = await redis.zrangebyscore('users', uid, uid)
        if (!_uid.length) {
            redis.zadd('users', uid, userStringify(user));
        }
    })
}

const getTopicsCache = (cid, start, redis) => {
    return redis.zrevrange(`topics:${cid}`, start, start + 49)
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

// check if all caterogries are cached
const isCidCached = nestArray => {
    let result = false;
    nestArray.forEach(array => {
        if (array.length === 0) {
            result = false;
        } else {
            result = true;
        }
    })
    return result;
}