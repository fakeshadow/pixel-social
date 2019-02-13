'use strict'

const {
    addPost: addPostSchema,
    editPost: editPostSchema,
    getPosts: getPostsSchema,
    getUserPosts: getUserPostsSchema
} = require('./schemas')

module.exports = async function (fastify, opts) {
    // All APIs are under authentication here!
    fastify.addHook('preHandler', fastify.authPreHandler)

    fastify.get('/', { schema: getPostsSchema }, getPostsHandler)
    fastify.post('/', { schema: addPostSchema }, addPostHandler)
    fastify.put('/', { schema: editPostSchema }, editPostHandler)
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
    const { uid } = req.user
    const { toPid } = req.body
    const { postData } = req.body
    await this.postService.addPost(uid, toPid, postData)
    reply.code(204)
}

async function editPostHandler(req, reply) {
    const { uid } = req.user
    const { pid } = req.body
    const { postData } = req.body
    await this.postService.editPost(uid, pid, postData)
    reply.code(204)
}

async function getPostsHandler(req, reply) {
    const { uid } = req.user
    return this.postService.getPosts(uid)
}

async function getUserPostsHandler(req, reply) {
    const userIds = req.params.userIds.split(',')
    return this.postService.getPosts(userIds)
}