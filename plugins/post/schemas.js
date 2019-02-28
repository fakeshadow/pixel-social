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

// raw posts for building cache
const rawPostObject = {
    type: 'object',
    properties: {
        uid: {
            type: 'integer',
            minimum: 1
        },
        pid: {
            type: 'integer',
            minimum: 1
        },
        toTid: {
            type: 'integer',
            minimum: 1
        },
        toPid: {
            type: 'integer',
            minimum: 0
        },
        postContent: {
            type: 'string'
        },
        postCount: {
            type: 'integer',
            minimum: 0
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
            type: 'integer',
            minimum: 1
        },
        toTid: {
            type: 'integer',
            minimum: 1
        },
        toPid: {
            type: 'integer',
            minimum: 0
        },
        postContent: {
            type: 'string'
        },
        postCount: {
            type: 'integer',
            minimum: 0
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
                type: 'integer',
                minimum: 0,
            },
            toTid: {
                type: 'integer',
                minimum: 0,
            },
            toPid: {
                type: 'integer',
                minimum: 0,
            },
            lastPid: {
                type: 'integer',
                minimum: 0,
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
                type: 'integer',
                minimum: 0
            },
            toTid: {
                type: 'integer',
                minimum: 0
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
                type: 'integer',
                minimum: 1
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