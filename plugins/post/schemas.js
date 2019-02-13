'use strict'

const postObject = {
    type: 'object',
    properties: {
        _id: { type: 'string' },
        user: {
            type: 'object',
            properties: {
                _id: { type: 'string' },
                //username: { type: 'string' }
            }
        },
        text: { type: 'string' }
    }
}

const post = {
    body: {
        type: 'object',
        required: ['text'],
        properties: {
            text: { type: 'string', minLength: 8, maxLength: 144 }
        },
        additionalProperties: false
    }
}

const getUserPosts = {
    params: {
        type: 'object',
        required: ['userIds'],
        properties: {
            userIds: {
                type: 'string',
                pattern: '^[0-9a-fA-F]{24}(,[0-9a-fA-F]{24})?'
            }
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
    post,
    getPosts,
    getUserPosts
}