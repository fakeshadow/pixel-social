'use strict'

const {
    addTopic: addTopicSchema,
} = require('./schemas')

module.exports = async function (fastify, opts) {
    // All APIs are under authentication here!
    fastify.addHook('preHandler', fastify.authPreHandler)

    fastify.post('/', { schema: addTopicSchema }, addTopicHandler)
}

module.exports[Symbol.for('plugin-meta')] = {
    decorators: {
        fastify: [
            'authPreHandler',
            'topicService',
            'postService',
            'userService',
        ]
    }
}

async function addTopicHandler(req, reply) {
    const { uid } = req.user;
    const { titleData } = req.body;
    const { postData } = req.body;
    const pid = await this.postService.addPost(uid, 0, postData);
    await this.topicService.addTopic(uid, pid, titleData);
    reply.code(204)
}
