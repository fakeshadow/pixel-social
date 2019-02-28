'use strict'

const userObject = {
    type: 'object',
    require: ['uid', 'username', 'email', 'avatar'],
    properties: {
        uid: {
            type: 'integer'
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

// raw posts for building cache
const rawPostObject = {
    type: 'object',
    properties: {
        uid: {
            type: 'integer'
        },
        pid: {
            type: 'integer'
        },
        toTid: {
            type: 'integer'
        },
        toPid: {
            type: 'integer'
        },
        postContent: {
            type: 'string'
        },
        postCount: {
            type: 'integer'
        },
        createdAt: {
            type: 'string'
        }
    },
    additionalProperties: false
}

const postObject = {
    type: 'object',
    properties: {
        pid: {
            type: 'integer'
        },
        toTid: {
            type: 'integer'
        },
        toPid: {
            type: 'integer'
        },
        postContent: {
            type: 'string'
        },
        postCount: {
            type: 'integer'
        },
        createdAt: {
            type: 'string'
        },
        user: userObject,
    },
    additionalProperties: false
}

const getPosts = {
    body: {
        type: 'object',
        required: ['uid', 'toTid', 'toPid', 'lastPid'],
        properties: {
            uid: {
                type: 'integer'
            },
            toTid: {
                type: 'integer'
            },
            toPid: {
                type: 'integer'
            },
            lastPid: {
                type: 'integer'
            },
        },
        additionalProperties: false
    },
    response: {
        200: {
            type: 'array',
            items: postObject
        }
    }
}

const addPost = {
    body: {
        type: 'object',
        required: ['toPid', 'toTid', 'postContent'],
        properties: {
            toPid: {
                type: 'integer'
            },
            toTid: {
                type: 'integer'
            },
            postContent: {
                type: 'string',
                minLength: 8,
                maxLength: 255
            }
        },
        additionalProperties: false
    }
}

const editPost = {
    body: {
        type: 'object',
        required: ['pid', 'postContent'],
        properties: {
            pid: {
                type: 'integer'
            },
            postContent: {
                type: 'string',
                minLength: 8,
                maxLength: 255
            }
        },
        additionalProperties: false
    }
}

// const getPosts = {
//     body: {
//         type: 'object',
//         required: ['pid', 'postContent'],
//         properties: {
//             pid: { type: 'number' },
//             postContent: { type: 'string', minLength: 8, maxLength: 255 }
//         },
//         additionalProperties: false
//     },
//     response: {
//         200: {
//             type: 'array',
//             items: postObject
//         }
//     }
// }

module.exports = {
    addPost,
    editPost,
    getPosts,
    // for cache hook
    rawPostObject,
    postObject
}