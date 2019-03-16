'use strict'

const {
    addTopic: addTopicSchema,
    getTopics: getTopicsSchema,
} = require('./schemas')

module.exports = async function (fastify, opts) {

    fastify.register(async function (fastify) {
        fastify
            .addHook('preHandler', fastify.authPreHandler)
            .addHook('preHandler', fastify.topicPreHandler)
            .addHook('preSerialization', fastify.topicPreSerialHandler);
        fastify.post('/', { schema: getTopicsSchema }, getTopicsHandler)
    })

    fastify.register(async function (fastify) {
        fastify
            .addHook('preHandler', fastify.authPreHandler)
            .addHook('preSerialization', fastify.topicPreSerialHandler);
        fastify.post('/add', { schema: addTopicSchema }, addTopicHandler);
    })

    fastify.setErrorHandler((error, req, res) => {
        res.send(error);
    })
}

module.exports[Symbol.for('plugin-meta')] = {
    decorators: {
        fastify: [
            'authPreHandler',
            'topicPreHandler',
            'topicPreSerialHandler',
            'topicService',
            'postService',
        ]
    }
}

async function getTopicsHandler(req, reply) {
    const { cids, lastPostTime } = req.body;
    return this.topicService.getTopics(cids, lastPostTime);
}

async function addTopicHandler(req, reply) {
    const { uid } = req.user;
    const { postContent, cid, topicContent } = req.body;
    
    const postData = {
        'toPid': 0,
        'toTid': 0,
        'postContent': postContent
    }
    const topicData = {
        "cid": cid,
        "topicContent": topicContent,
    }
    return this.postService.addPost(uid, postData, topicData);
}