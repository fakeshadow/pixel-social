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

const rawTopicObject = {
    type: 'object',
    properties: {
        uid: {
            type: 'integer',
            minimum: 1
        },
        tid: {
            type: 'integer',
            minimum: 1
        },
        cid: { type: 'string' },
        mainPid: {
            type: 'integer',
            minimum: 1
        },
        topicContent: { type: 'string' },
        postCount: {
            type: 'integer',
            minimum: 0
        },
        lastPostTime: { type: 'string' },
    },
    additionalProperties: false
}


const topicObject = {
    type: 'object',
    properties: {
        tid: {
            type: 'integer',
            minimum: 1
        },
        cid: { type: 'string' },
        mainPid: {
            type: 'integer',
            minimum: 1
        },
        topicContent: { type: 'string' },
        postCount: {
            type: 'integer',
            minimum: 0
        },
        lastPostTime: { type: 'string' },
        user: userObject,
    },
    additionalProperties: false
}

const getTopics = {
    body: {
        type: 'object',
        required: ['cid', 'lastPostTime'],
        properties: {
            cid: { type: 'string' },
            lastPostTime: { type: 'string' },
        },
        additionalProperties: false
    },
    response: {
        200: {
            type: 'array',
            items: topicObject
        }
    }
}

const addTopic = {
    body: {
        type: 'object',
        required: ['cid', 'topicContent', 'postContent'],
        properties: {
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