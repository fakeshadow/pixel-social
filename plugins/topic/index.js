'use strict'

const {
    addTopic: addTopicSchema,
    getTopics: getTopicsSchema,
} = require('./schemas')

module.exports = async function (fastify, opts) {
    // All APIs are under authentication here!
    fastify
        .addHook('preHandler', fastify.authPreHandler)
        .addHook('preHandler', fastify.cachePreHandler)
        .addHook('preSerialization', fastify.cachePreSerialHandler);

    fastify.post('/get', { schema: getTopicsSchema }, getTopicsHandler)
    fastify.post('/add', { schema: addTopicSchema }, addTopicHandler)

    fastify.setErrorHandler((error, req, res) => {
        res.send(error);
    })
}

module.exports[Symbol.for('plugin-meta')] = {
    decorators: {
        fastify: [
            'authPreHandler',
            'topicService',
            'postService',
            'userService'
        ]
    }
}

async function getTopicsHandler(req, reply) {
    const {
        cids,
        page
    } = req.body;
    return await this.topicService.getTopics(cids, page);
}

async function addTopicHandler(req, reply) {
    const {
        uid
    } = req.user;
    // topic is binding to a post which has 0 topid and totid. The pid of this post is write into topic's mainpid.
    const postData = {
        'toPid': 0,
        'toTid': 0,
        'postContent': req.body.postContent
    }
    const pid = await this.postService.addPost(uid, postData);
    const topicData = {
        "mainPid": pid,
        "cid": req.body.cid,
        "topicContent": req.body.topicContent,
    }
    await this.topicService.addTopic(uid, topicData);
    reply.code(204)
}