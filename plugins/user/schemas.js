'use strict'

const userProfileOutput = {
    type: 'object',
    require: ['uid', 'username', 'email'],
    properties: {
        uid: { type: 'number' },
        username: { type: 'string' },
        email: { type: 'string' },
    },
    additionalProperties: false
}

const registration = {
    body: {
        type: 'object',
        required: ['username', 'email', 'password'],
        properties: {
            username: {
                type: 'string'
            },
            email: {
                type: 'string'
            },
            password: {
                type: 'string'
            }
        },
        additionalProperties: false
    },
    response: {
        200: {
            type: 'object',
            required: ['userId'],
            properties: {
                userId: { type: 'string' }
            },
            additionalProperties: false
        }
    }
}

const login = {
    body: {
        type: 'object',
        require: ['username', 'password'],
        properties: {
            username: { type: 'string' },
            password: { type: 'string' }
        },
        additionalProperties: false
    },
    response: {
        200: {
            type: 'object',
            require: ['jwt'],
            properties: {
                jwt: { type: 'string' }
            },
            additionalProperties: false
        }
    }
}

// const search = {
//     querystring: {
//         type: 'object',
//         require: ['search'],
//         properties: {
//             search: { type: 'string' }
//         },
//         additionalProperties: false
//     },
//     response: {
//         200: {
//             type: 'array',
//             items: userProfileOutput
//         }
//     }
// }

const getProfile = {
    params: {
        type: 'object',
        required: ['uid'],
        properties: {
            uid: {
                type: 'number'
            }
        }
    },
    response: {
        200: userProfileOutput
    }
}

module.exports = {
    registration,
    login,
    // search,
    getProfile
}