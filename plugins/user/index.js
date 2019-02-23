'use strict'

const {
    login: loginSchema,
    registration: registrationSchema,
    getProfile: getProfileSchema
} = require('./schemas')

module.exports = async (fastify, opts) => {
    fastify.post('/login', { schema: loginSchema }, loginHandler)
    fastify.post('/register', { schema: registrationSchema }, registerHandler)

    fastify.register(async function (fastify) {
        fastify.addHook('preHandler', fastify.authPreHandler);
        fastify.addHook('preHandler', fastify.cachePreHandler);
        fastify.addHook('preSerialization', fastify.cachePreSerialHandler);

        fastify.post('/', { schema: getProfileSchema }, userHandler)
    })

    fastify.setErrorHandler((error, req, res) => {
        res.send(error);
    })
}

module.exports[Symbol.for('plugin-meta')] = {
    decorators: {
        fastify: [
            'authPreHandler',
            'userService',
            'jwt'
        ]
    }
}

async function loginHandler(req, reply) {
    const { username, password } = req.body
    const user = await this.userService.login(username, password)
    return { jwt: this.jwt.sign(user) }
}

async function registerHandler(req, reply) {
    const { username, email, password } = req.body
    const userId = await this.userService.register(username, email, password)
    return { userId: userId }
}

async function userHandler(req, reply) {
    const { uid } = req.body
    return this.userService.getProfile(uid);
}