'use strict'
const fastify = require('fastify')();
const fp = require('fastify-plugin');
const morgan = require('morgan');

const UserService = require('./plugins/user/service');

require('dotenv').config();

fastify.use(morgan('common'));

function transformStringIntoObjectId(str) {
    return new this.mongo.ObjectId(str)
}

const decorateFastifyInstance = async (fastify) => {
    const db = fastify.mongo.db

    const userCollection = await db.createCollection('users')
    const userService = new UserService(userCollection)
    await userService.ensureIndexes(db)
    fastify
        .decorate('userService', userService)
        .decorate('authPreHandler', async (req, res) => {
            try {
                await req.jwtVerify()
            } catch (err) {
                res.send(err)
            }
        })
        .decorate('transformStringIntoObjectId', transformStringIntoObjectId)
}

fastify
    .register(require('fastify-mongodb'), { url: process.env.DATABASE, useNewUrlParser: true })
    .register(require('fastify-jwt'), { secret: process.env.JWT, algorithms: ['RS256'] })
    .register(fp(decorateFastifyInstance))
    .register(require('./plugins/user'), { prefix: '/api/user' })

const start = async () => {
    try {
        await fastify.listen(3100)
        console.log(`server listening on ${fastify.server.address().port}`)
    } catch (err) {
        console.log(err)
        process.exit(1)
    }
}
start()

