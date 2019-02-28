'use strict'

const userObject = {
    type: 'object',
    require: ['uid', 'username', 'email', 'avatar'],
    properties: {
        uid: {
            type: 'integer',
            minimum: 1
        },
        username: {
            type: 'string'
        },
        email: {
            type: 'string'
        },
        avatar: {
            type: 'string'
        }
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
    response: 200

}

const login = {
    body: {
        type: 'object',
        require: ['username', 'password'],
        properties: {
            username: {
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
            require: ['jwt'],
            properties: {
                jwt: {
                    type: 'string'
                }
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
    body: {
        type: 'object',
        required: ['uid'],
        properties: {
            uid: {
                type: 'integer',
                minimum: 1
            }
        },
        additionalProperties: false
    },
    response: {
        200: userObject
    }
}

const updateProfile = {
    body: {
        require: ['avatar'],
        properties: {
            avatar: {
                type: 'string'
            }
        },
        additionalProperties: false
    },
}

module.exports = {
    registration,
    login,
    // search,
    getProfile,
    updateProfile,
    userObject,
}