'use strict'

const { addPost: addPostSchema, editPost: editPostSchema, getPosts: getPostsSchema, } = require('./schemas')

module.exports = async function (fastify, opts) {

    fastify.register(async function (fastify) {
        fastify
            .addHook('preHandler', fastify.authPreHandler)
            .addHook('preHandler', fastify.postPreHandler)
            .addHook('preSerialization', fastify.postPreSerialHandler);
        fastify.post('/', { schema: getPostsSchema }, getPostsHandler);
    })

    fastify.register(async function (fastify) {
        fastify
            .addHook('preHandler', fastify.authPreHandler)
            .addHook('preSerialization', fastify.postPreSerialHandler);
        fastify.post('/add', { schema: addPostSchema }, addPostHandler);
        fastify.post('/edit', { schema: editPostSchema }, editPostHandler);
    })

    fastify.setErrorHandler((error, req, res) => {
        res.send(error);
    })
}

module.exports[Symbol.for('plugin-meta')] = {
    decorators: {
        fastify: [
            'authPreHandler',
            'postPreHandler',
            'postPreSerialHandler',
            'postService'
        ]
    }
}

async function addPostHandler(req, reply) {
    const { uid } = req.user;
    const { toTid, toPid, postContent } = req.body;
    const postData = {
        'toTid': toTid,
        'toPid': toPid,
        'postContent': postContent
    }
    return this.postService.addPost(uid, postData, null);
}

async function editPostHandler(req, reply) {
    const { uid } = req.user
    const { pid, postContent } = req.body;
    const postData = {
        "pid": pid,
        "postContent": postContent
    }
    return this.postService.editPost(uid, postData)
}

async function getPostsHandler(req, reply) {
    const { uid } = req.user;
    return this.postService.getPosts(uid, req.body);
}