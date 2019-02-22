'use strict'

const fastify = require('fastify')();
const fp = require('fastify-plugin');
const morgan = require('morgan');

const { authPreHook } = require('./hooks/authHooks');
const { cachePreHook, cachePreSerialHook } = require('./hooks/cacheHooks');
const UserService = require('./plugins/user/service');
const PostService = require('./plugins/post/service');
const TopicService = require('./plugins/topic/service');
const FileService = require('./plugins/file/service');

require('dotenv').config();

fastify.use(morgan('tiny'));

const decorateFastifyInstance = async (fastify) => {
    const db = fastify.mongo.db

    const globalCollection = await db.createCollection('global')
    const userCollection = await db.createCollection('users')
    const postCollection = await db.createCollection('posts')
    const topicCollection = await db.createCollection('topics')

    const userService = new UserService(userCollection, globalCollection);
    const topicService = new TopicService(topicCollection, userCollection, globalCollection);
    const postService = new PostService(topicCollection, postCollection, userCollection, globalCollection);
    const fileService = new FileService(postCollection);

    await userService.ensureIndexes(db);
    await postService.ensureIndexes(db);
    await topicService.ensureIndexes(db);

    fastify
        .decorate('userService', userService)
        .decorate('postService', postService)
        .decorate('topicService', topicService)
        .decorate('fileService', fileService)
        .decorate('authPreHandler', authPreHook)
        .decorate('cachePreHandler', cachePreHook)
        .decorate('cachePreSerialHandler', cachePreSerialHook)
}

fastify
    .register(require('fastify-mongodb'), { url: process.env.MONGO, useNewUrlParser: true })
    .register(require('fastify-jwt'), { secret: process.env.JWT, algorithms: ['RS256'] })
    .register(require('fastify-multipart'))
    .register(require('fastify-redis'), { host: process.env.REDIS_IP, port: process.env.REDIS_PORT, family: 4, password: process.env.REDIS_PASS })
    .register(fp(decorateFastifyInstance))
    .register(require('./plugins/user'), { prefix: '/api/user' })
    .register(require('./plugins/post'), { prefix: '/api/post' })
    .register(require('./plugins/topic'), { prefix: '/api/topic' })
    .register(require('./plugins/file'), { prefix: '/api/file' })
    .register(require('./plugins/test'), { prefix: '/api/test' })

const start = async () => {
    try {
        await fastify.listen(3100, "192.168.1.197")
        console.log(`server listening on ${fastify.server.address().port}`)
    } catch (e) {
        console.log(e)
        process.exit(1)
    }
}
start()

