'use strict'

const {
    login: loginSchema,
    registration: registrationSchema,
    getProfile: getProfileSchema,
    editProfile: updateProfileSchema
} = require('./schemas')

module.exports = async (fastify, opts) => {
    fastify.post('/register', { schema: registrationSchema }, registerHandler)

    fastify.register(async function (fastify) {
        fastify.addHook('preSerialization', fastify.userPreSerialHandler);
        fastify.post('/login', { schema: loginSchema }, loginHandler);
    })

    fastify.register(async function (fastify) {
        fastify
            .addHook('preHandler', fastify.authPreHandler)
            .addHook('preHandler', fastify.userPreHandler)
            .addHook('preSerialization', fastify.userPreSerialHandler);
        fastify.post('/', { schema: getProfileSchema }, userHandler);
    })

    fastify.register(async function (fastify) {
        fastify
            .addHook('preHandler', fastify.authPreHandler)
            .addHook('preSerialization', fastify.userPreSerialHandler);
        fastify.post('/update', { schema: updateProfileSchema }, updateUserHandler)
    })

    fastify.setErrorHandler((error, req, res) => {
        res.send(error);
    })
}

module.exports[Symbol.for('plugin-meta')] = {
    decorators: {
        fastify: [
            'authPreHandler',
            'userPreHandler',
            'userPreSerialHandler',
            'userService',
            'jwt'
        ]
    }
}

async function loginHandler(req, reply) {
    const user = req.body.username;
    const pass = req.body.password;
    const userData = await this.userService.login(user, pass);
    const { uid, username, email, avatar } = userData;
    return { jwt: this.jwt.sign(uid), uid, username, email, avatar }
}

async function registerHandler(req, reply) {
    const { username, email, password } = req.body
    await this.userService.register(username, email, password)
    return { 'message': 'success please login' }
}

async function userHandler(req, reply) {
    const { uid } = req.body
    return this.userService.getProfile(uid);
}

// currently only allow change avatar
async function updateUserHandler(req, reply) {
    const { uid } = req.user;
    const { avatar } = req.body;
    const userData = {
        avatar: avatar
    }
    return this.userService.updateProfile(uid, userData);
}