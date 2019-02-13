'use strict'

const {
    post: postSchema,
    getPosts: getPostsSchema,
    getUserPosts: getUserPostsSchema
} = require('./schemas')

module.exports = async function (fastify, opts) {
    // All APIs are under authentication here!
    fastify.addHook('preHandler', fastify.authPreHandler)

    fastify.post('/', { schema: postSchema }, addPostHandler)
    fastify.get('/', { schema: getPostsSchema }, getPostsHandler)
    fastify.get('/:userIds', { schema: getUserPostsSchema }, getUserPostsHandler)
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
    console.log(req.user);
    console.log(req.body);
    const { text } = req.body
    await this.postService.addPost(req.user, text)
    reply.code(204)
}

async function getPostsHandler(req, reply) {
    return this.postService.fetchPosts([req.user._id])
}

async function getUserPostsHandler(req, reply) {
    const userIds = req.params.userIds.split(',')
    return this.postService.fetchPosts(userIds)
}