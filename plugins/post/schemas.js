'use strict'

const postObject = {
    type: 'object',
    properties: {
        _id: { type: 'string' },
        uid: { type: 'number' },
        pid: { type: 'number' },
        toPid: { type: 'number' },
        postData: { type: 'string' },
    }
}

const addPost = {
    body: {
        type: 'object',
        required: ['toPid', 'postData'],
        properties: {
            toPid: { type: 'number' },
            postData: { type: 'string', minLength: 8, maxLength: 255 }
        },
        additionalProperties: false
    }
}

const editPost = {
    body: {
        type: 'object',
        required: ['pid', 'postData'],
        properties: {
            pid: { type: 'number' },
            postData: { type: 'string', minLength: 8, maxLength: 255 }
        },
        additionalProperties: false
    }
}

const getUserPosts = {
    params: {
        type: 'object',
        required: ['uid'],
        properties: {
            uid: { type: 'number' }
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

const getPosts = {
    response: {
        200: {
            type: 'array',
            items: postObject
        }
    }
}

module.exports = {
    addPost,
    editPost,
    getPosts,
    getUserPosts
}