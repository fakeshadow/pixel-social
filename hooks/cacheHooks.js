'use strict'

const fastJson = require('fast-json-stringify');

// use schemas to speed up stringify speed
const { mapUid, alterPosts, alterTopics, parseCache } = require('../util/sortIds');
const arrayFlatten = require('../util/arrayflatten');
const { rawPostObject, postObject } = require('../plugins/post/schemas');
const { rawTopicObject, topicObject } = require('../plugins/topic/schemas');

const { userObject } = require('../plugins/user/schemas');

const postsCache = {
    type: 'object',
    required: ['cache', 'database'],
    properties: {
        cache: {
            type: 'array',
            //ues postObject schema here as we want to send user detailed info to user.
            items: postObject
        },
        database: {
            type: 'array',
            items: postObject
        },
    },
    additionalProperties: false
}

const topicsCache = {
    type: 'object',
    required: ['cache', 'database'],
    properties: {
        cache: {
            type: 'array',
            //ues topicObject schema here as we want to send user detailed info to user.
            items: topicObject
        },
        database: {
            type: 'array',
            items: topicObject
        },
    },
    additionalProperties: false
}

const postStringify = fastJson(rawPostObject);
const postsStringify = fastJson(postsCache);
const topicStringify = fastJson(rawTopicObject);
const topicsStringify = fastJson(topicsCache);
const userStringify = fastJson(userObject);

//check if the request content are in cache
async function cachePreHook(req, res) {
    try {
        const { type } = req.body;

        // toTid:id is a sortedset with all post pids for a topic.
        // posts is a list with all the details of a post.
        // users is a sortedset with all user detail info.
        // topics is a sortedset with all topics info.

        // need further work on dealing with uid maps. currently reading all uids.

        if (type === 'getPosts') {
            const { toTid, page } = req.body;
            const _page = parseInt(page, 10);
            const _toTid = parseInt(toTid, 10);
            if (_page <= 0) throw new Error('wrong page');
            const start = (_page - 1) * 50

            const postsCache = await this.redis.zrange(`toTid:${_toTid}`, start, start + 49);

            if (postsCache.length) {
                const posts = await parseCache(postsCache);
                const uidsMap = await mapUid(posts);
                const usersCache = await getUsersCache(uidsMap, this.redis);
                const users = await parseCache(usersCache);
                const postsFinal = await alterPosts(posts, users);
                const string = postsStringify({ 'cache': postsFinal, 'database': [] });
                return res.send(string);
            }

        } else if (type === 'getUser') {
            const { uid } = req.body;
            const _uid = parseInt(uid, 10);
            const userCache = await this.redis.zrangebyscore('users', _uid, _uid + 1);
            if (userCache.length) {
                const user = await parseCache(userCache);
                const userFinal = await userStringify(user[0]);
                return res.send(userFinal);
            }

        } else if (type === 'getTopics') {
            // await this.redis.flushall();

            const { cids, page } = req.body;
            const _page = parseInt(page, 10);
            if (_page <= 0) throw new Error('wrong page');
            const start = (_page - 1) * 50;

            const promises = [];
            cids.forEach(cid => promises.push(getTopicsCache(cid, start, this.redis)));
            const topicsUnsort = await Promise.all(promises);
            const isCached = isCidCached(topicsUnsort);
            if (!isCached) return;

            const topicsCache = arrayFlatten(topicsUnsort);
            if (topicsCache.length) {
                const topics = await parseCache(topicsCache);
                const uidsMap = await mapUid(topics);
                const usersCache = await getUsersCache(uidsMap, this.redis);
                const users = await parseCache(usersCache);
                const topicsFinal = await alterTopics(topics, users);
                if (topicsFinal.length) {
                    const string = topicsStringify({ 'cache': topicsFinal, 'database': [] })
                    return res.send(string);
                }
            }
        }
    } catch (err) {
        res.send(err)
    }
}

async function cachePreSerialHook(req, res, payload) {
    try {
        const { type } = req.body;

        if (type === 'getPosts') {
            const { toTid } = req.body;
            const { cache, database } = payload;
            if (!database.length) return payload;

            cache.forEach(post => {
                const { pid } = post;
                return this.redis.zadd(`toTid:${toTid}`, pid, postStringify(post))
            })

            addUsersCache(database, this.redis);
            return { 'cache': [], 'database': database };

        } else if (type === 'getUser') {
            const { uid } = payload;
            const _uid = await this.redis.zrangebyscore('users', uid, uid)
            if (!_uid.length) {
                this.redis.zadd('users', uid, userStringify(payload));
            }
            return payload;

        } else if (type === 'getTopics') {
            const { cache, database } = payload;
            if (!database.length) return payload;

            cache.forEach(async topic => {
                const { cid, lastPostTime } = topic;
                const timeString = new Date(lastPostTime).toISOString();
                const timeScore = Date.parse(timeString);
                const cachedTopic = await this.redis.zrangebyscore(`topics:${cid}`, timeScore, timeScore);
                if (!cachedTopic.length) {
                    const string = topicStringify(topic)
                    await this.redis.zadd(`topics:${cid}`, timeScore, string);
                }
            })
            addUsersCache(database, this.redis);
            return { 'cache': [], 'database': database };

        } else if (type === 'addPost' || type === 'editPost') {

            const { toTid, pid } = payload
            const cachedPost = await this.redis.zrangebyscore(`toTid:${toTid}`, pid, pid)
            if (!cachedPost.length) {
                this.redis.zadd(`toTid:${toTid}`, pid, postStringify(payload))
                return { message: 'success' }
            }
            if (!cachedPost.length) {
            }

        } else if (type === 'addTopic') {
            console.log(payload);
            return 'success';
        }
    } catch (err) {
        res.send(err)
    }
}

module.exports = {
    cachePreHook,
    cachePreSerialHook
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