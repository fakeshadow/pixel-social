'use strict'

const userProfileOutput = {
    type: 'object',
    require: ['_id', 'username'],
    properties: {
        _id: { type: 'string' },
        username: { type: 'string' }
    },
    additionalProperties: false
}

const registration = {
    body: {
        type: 'object',
        required: ['username', 'email', 'password'],
        properties: {
            uid: {
                type: 'number'
            },
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
        required: ['userId'],
        properties: {
            userId: {
                type: 'string',
                pattern: '^[0-9a-fA-F]{24}'
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