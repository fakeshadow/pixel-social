'use strict'

const fastify = require('fastify')();
const fp = require('fastify-plugin');
const morgan = require('morgan');

const { authPreHook } = require('./hooks/authHooks');
const UserService = require('./plugins/user/service');
const PostService = require('./plugins/post/service');
const TopicService = require('./plugins/topic/service');

require('dotenv').config();

fastify.use(morgan('common'));

const decorateFastifyInstance = async (fastify) => {
    const db = fastify.mongo.db

    const globalCollection = await db.createCollection('global')
    const userCollection = await db.createCollection('users')
    const postCollection = await db.createCollection('posts')
    const topicCollection = await db.createCollection('topics')

    const postService = new PostService(topicCollection, postCollection, userCollection, globalCollection);
    const userService = new UserService(userCollection, globalCollection);
    const topicService = new TopicService(topicCollection, userCollection, globalCollection);

    await userService.ensureIndexes(db)
    await postService.ensureIndexes(db)
    await topicService.ensureIndexes(db)

    fastify
        .decorate('userService', userService)
        .decorate('postService', postService)
        .decorate('topicService', topicService)
        .decorate('authPreHandler', authPreHook)
        .decorate('transformStringIntoObjectId', function (str) { return new this.mongo.ObjectId(str) })
}

fastify
    .register(require('fastify-mongodb'), { url: process.env.DATABASE, useNewUrlParser: true })
    .register(require('fastify-jwt'), { secret: process.env.JWT, algorithms: ['RS256'] })
    .register(fp(decorateFastifyInstance))
    .register(require('./plugins/user'), { prefix: '/api/user' })
    .register(require('./plugins/post'), { prefix: '/api/post' })
    .register(require('./plugins/topic'), { prefix: '/api/topic' })

const start = async () => {
    try {
        await fastify.listen(3100, "192.168.1.197")
        console.log(`server listening on ${fastify.server.address().port}`)
    } catch (err) {
        console.log(err)
        process.exit(1)
    }
}
start()

