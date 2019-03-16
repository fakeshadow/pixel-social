'use strict'

const path = require('path');
const fastify = require('fastify')();
const fp = require('fastify-plugin');

const { authPreHandler } = require('./hooks/auth');
const { userPreHandler, userPreSerialHandler } = require('./hooks/user');
const { postPreHandler, postPreSerialHandler } = require('./hooks/post');
const { topicPreHandler, topicPreSerialHandler } = require('./hooks/topic');

const UserService = require('./plugins/user/service');
const PostService = require('./plugins/post/service');
const TopicService = require('./plugins/topic/service');
const FileService = require('./plugins/file/service');
const CacheService = require('./plugins/cache/service');

require('dotenv').config();

fastify.use(require('morgan')('tiny'));

const decorateFastifyInstance = async fastify => {
    const db = fastify.mongo.db

    const globalCollection = await db.createCollection('global')
    const userCollection = await db.createCollection('users')
    const postCollection = await db.createCollection('posts')
    const topicCollection = await db.createCollection('topics')

    const userService = new UserService(userCollection, globalCollection);
    const topicService = new TopicService(topicCollection, globalCollection);
    const postService = new PostService(topicCollection, postCollection, globalCollection);
    const fileService = new FileService(postCollection);
    const cacheService = new CacheService(fastify.redis);

    await userService.ensureIndexes(db);
    await postService.ensureIndexes(db);
    await topicService.ensureIndexes(db);

    fastify
        .decorate('userService', userService)
        .decorate('postService', postService)
        .decorate('topicService', topicService)
        .decorate('fileService', fileService)
        .decorate('cacheService', cacheService)
        .decorate('authPreHandler', authPreHandler)
        .decorate('userPreHandler', userPreHandler)
        .decorate('userPreSerialHandler', userPreSerialHandler)
        .decorate('postPreHandler', postPreHandler)
        .decorate('postPreSerialHandler', postPreSerialHandler)
        .decorate('topicPreHandler', topicPreHandler)
        .decorate('topicPreSerialHandler', topicPreSerialHandler)
}

const connectToDatabases = async fastify => {
    fastify
        .register(require('fastify-mongodb'), { url: process.env.MONGO, useNewUrlParser: true })
        .register(require('fastify-redis'), { host: process.env.REDIS_IP, port: process.env.REDIS_PORT, family: 4, password: process.env.REDIS_PASS })
}

fastify
    .register(require('fastify-jwt'), { secret: process.env.JWT, algorithms: ['RS256'] })
    .register(require('fastify-multipart'))
    .register(require('fastify-static'), { root: path.join(__dirname, 'public'), prefix: '/public/', })
    .register(fp(connectToDatabases))
    .register(fp(decorateFastifyInstance))
    .register(require('./plugins/user'), { prefix: '/api/user' })
    .register(require('./plugins/post'), { prefix: '/api/post' })
    .register(require('./plugins/topic'), { prefix: '/api/topic' })
    .register(require('./plugins/file'), { prefix: '/api/file' })

const start = async () => {
    try {
        await fastify.listen(process.env.PORT || 3100, process.env.IP || '127.0.0.1')
        console.log(`server listening on ${fastify.server.address().port}`)
    } catch (e) {
        console.log(e)
        process.exit(1)
    }
}

start();