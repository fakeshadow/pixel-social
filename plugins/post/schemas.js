'use strict'

const userObject = {
    type: 'object',
    properties: {
        uid: { type: 'number' },
        username: { type: 'string' },
    },
    additionalProperties: false
}

const postObject = {
    type: 'object',
    properties: {
        user: userObject,
        pid: { type: 'number' },
        toTid: { type: 'number' },
        toPid: { type: 'number' },
        postContent: { type: 'string' },
        postCount: { type: 'number' },
        createdAt: { type: 'string' },
    }
}

const getPosts = {
    body: {
        type: 'object',
        required: ['uid', 'toTid', 'toPid', 'page'],
        properties: {
            uid: { type: 'number' },
            toTid: { type: 'number' },
            toPid: { type: 'number' },
            page: { type: 'number' },
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
            toPid: { type: 'number' },
            toTid: { type: 'number' },
            postContent: { type: 'string', minLength: 8, maxLength: 255 }
        },
        additionalProperties: false
    }
}

const editPost = {
    body: {
        type: 'object',
        required: ['pid', 'postContent'],
        properties: {
            pid: { type: 'number' },
            postContent: { type: 'string', minLength: 8, maxLength: 255 }
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
}