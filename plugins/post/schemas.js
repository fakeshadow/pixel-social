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
        },
        isTopicMain: {
            type: 'integer'
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
        required: ['type', 'uid', 'toTid', 'toPid', 'page'],
        properties: {
            type: {
                type: 'string'
            },
            uid: {
                type: 'integer'
            },
            toTid: {
                type: 'integer'
            },
            toPid: {
                type: 'integer'
            },
            page: {
                type: 'integer'
            },
        },
        additionalProperties: false
    },
    response: {
        200: {
            type: 'object',
            required: ['cache', 'database'],
            properties: {
                cache: {
                    type: 'array',
                    items: rawPostObject
                },
                database: {
                    type: 'array',
                    items: postObject
                },
            },
            additionalProperties: false
        }
    }
}

const addPost = {
    body: {
        type: 'object',
        required: ['type', 'toPid', 'toTid', 'postContent'],
        properties: {
            type: {
                type: 'string'
            },
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
        required: ['type', 'pid', 'postContent'],
        properties: {
            type: {
                type: 'string'
            },
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