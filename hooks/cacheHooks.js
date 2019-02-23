'use strict'

const fastJson = require('fast-json-stringify');

// use schemas to speed up stringify speed
const { mapUid, alterPosts, alterTopics, parseCache } = require('../util/sortIds');
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
        // await this.redis.flushall();
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
                const usersCache = await this.redis.zrange(`users`, 0, -1);
                let posts, users;
                await Promise.all([
                    posts = await parseCache(postsCache),
                    users = await parseCache(usersCache)
                ])
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

            // work on here
        } else if (type === 'getTopics') {
            const { cids, page } = req.body;
            const _page = parseInt(page, 10);
            if (_page <= 0) throw new Error('wrong page');
            const start = (_page - 1) * 50;

            const topicsCache = await getTopicsCache(cids, start, this.redis);

            if (topicsCache.length) {
                const topics = await parseCache(topicsCache);
                const uidsMap = await mapUid(topics);
                const usersCache = await getUsersCache(uidsMap, this.redis);
                const users = await parseCache(usersCache);
                const topicsFinal = await alterTopics(topics, users);

                const string = topicsStringify({ 'cache': topicsFinal, 'database': [] })
                return res.send(string);
            }
        }
    } catch (err) {
        res.send(err)
    }
}

async function cachePreSerialHook(req, res, payload) {
    try {
        const { type } = req.body;

        if (type === 'getPosts' || type === 'addPost' || type === 'editPost') {
            const { toTid } = req.body;
            const { cache, database } = payload;

            cache.forEach(post => {
                const { pid } = post;
                return this.redis.zadd(`toTid:${toTid}`, pid, postStringify(post))
            })

            // update users detialed info set with payload.databse
            database.forEach(async post => {
                const { user } = post;
                const { uid } = user;
                const _uid = await this.redis.zrangebyscore('users', uid, uid + 1)
                if (!_uid.length) {
                    return this.redis.zadd('users', uid, userStringify(user));
                }
            })
            return { 'cache': [], 'database': database };

        } else if (type === 'getUser') {

            const { uid } = payload;
            const _uid = await this.redis.zrangebyscore('users', uid, uid + 1)
            if (!_uid.length) {
                this.redis.zadd('users', uid, userStringify(payload));
            }
            return payload;
            //    work on here;
        } else if (type === 'getTopics') {
            const { cache, database } = payload;
            cache.forEach(async topic => {
                const { cid, lastPostTime } = topic;
                const timeScore = new Date(lastPostTime).getTime();
                const _timeScore = await this.redis.zrange(`topics:${cid}`, timeScore, timeScore + 1)
                if (!_timeScore.length) {
                    return this.redis.zadd(`topics:${cid}`, timeScore, topicStringify(topic));
                }
                if (_timeScore.length) {
                }
            })

            return { 'cache': [], 'database': database };

        } else if (type === 'addTopic') {

        } else if (type === 'updateProfile') {

        }
    } catch (err) {
        res.send(err)
    }
}

module.exports = {
    cachePreHook,
    cachePreSerialHook
}


// need to study why there is a bug in the async for each
const getTopicsCache = (cids, start, redis) => {
    return new Promise((resolve) => {
        let topicsCache = [];
        cids.forEach(async (cid, index) => {
            const temp = await redis.zrange(`topics:${cid}`, start, start + 49)
            topicsCache = topicsCache.concat(temp);
            if (index === cids.length - 1) {
                return resolve(topicsCache);
            }
        })
    })
}

const getUsersCache = (uidsMap, redis) => {
    return new Promise(resolve => {
        let usersCache = [];
        uidsMap.forEach(async (uid, index) => {
            const user = await redis.zrangebyscore('users', uid, uid + 1);
            usersCache = usersCache.concat(user);
            if (index === uidsMap.length - 1) {
                return resolve(usersCache);
            }
        })
    })
}

