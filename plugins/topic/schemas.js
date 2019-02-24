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

const rawTopicObject = {
    type: 'object',
    properties: {
        uid: { type: 'integer' },
        tid: { type: 'integer' },
        cid: { type: 'string' },
        mainPid: { type: 'integer' },
        topicContent: { type: 'string' },
        postCount: { type: 'integer' },
        lastPostTime: { type: 'string' },
    },
    additionalProperties: false
}


const topicObject = {
    type: 'object',
    properties: {
        tid: { type: 'integer' },
        cid: { type: 'string' },
        mainPid: { type: 'integer' },
        topicContent: { type: 'string' },
        postCount: { type: 'integer' },
        lastPostTime: { type: 'string' },
        user: userObject,
    },
    additionalProperties: false
}

const getTopics = {
    body: {
        type: 'object',
        required: ['type', 'cids', 'page'],
        properties: {
            type: { type: 'string' },
            cids: { type: 'array', items: { type: 'string' } },
            page: { type: 'integer' },
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
                    items: rawTopicObject
                },
                database: {
                    type: 'array',
                    items: topicObject
                },
            },
            additionalProperties: false
        }
    }
}

const addTopic = {
    body: {
        type: 'object',
        required: ['type', 'cid', 'topicContent', 'postContent'],
        properties: {
            type: { type: 'string' },
            cid: { type: 'string' },
            topicContent: { type: 'string', minLength: 8, maxLength: 255 },
            postContent: { type: 'string', minLength: 8, maxLength: 255 }
        },
        additionalProperties: false
    }
}

module.exports = {
    addTopic,
    getTopics,
    topicObject,
    rawTopicObject
}