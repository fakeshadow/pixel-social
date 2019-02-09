'use strict'

const {
    //login: loginSchema,
    registration: registrationSchema,
    // search: searchSchema,
    //getProfile: getProfileSchema
} = require('./schemas')

module.exports = async (fastify, opts) => {
    fastify.post('/register', { schema: registrationSchema }, registerHandler)

    fastify.setErrorHandler((error, req, res) => {
        res.send(error);
    })
}

module.exports[Symbol.for('plugin-meta')] = {
    decorators: {
        fastify: [
            'userService'
        ]
    }
}

async function registerHandler(req, res) {
    const { username, email, password } = req.body
    const userId = await this.userService.register(username, email, password)
    return { userId: userId }
}