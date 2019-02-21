'use strict'

const {
    addPost: addPostSchema,
    editPost: editPostSchema,
    getPosts: getPostsSchema,
} = require('./schemas')

module.exports = async function (fastify, opts) {
    // All APIs are under authentication here!
    fastify.addHook('preHandler', fastify.authPreHandler);

    fastify.post('/get', { schema: getPostsSchema }, getPostsHandler);
    fastify.post('/add', { schema: addPostSchema }, addPostHandler);
    fastify.post('/edit', { schema: editPostSchema }, editPostHandler);
}

module.exports[Symbol.for('plugin-meta')] = {
    decorators: {
        fastify: [
            'authPreHandler',
            'postService'
        ]
    }
}

async function addPostHandler(req, reply) {
    const { uid } = req.user
    const postData = {
        'toTid': req.body.toTid,
        'toPid': req.body.toPid,
        'postContent': req.body.postContent
    }
    await this.postService.addPost(uid, postData)
    reply.code(204)
}

async function editPostHandler(req, reply) {
    const { uid } = req.user
    const postData = {
        "pid": req.body.pid,
        "postContent": req.body.postContent
    }
    await this.postService.editPost(uid, postData)
    reply.code(204)
}

async function getPostsHandler(req, reply) {
    const { uid } = req.user;
    return this.postService.getPosts(uid, req.body);
}