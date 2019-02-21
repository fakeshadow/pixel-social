'use strict'

const userObject = {
    type: 'object',
    properties: {
        uid: { type: 'number' },
        username: { type: 'string' },
    },
    additionalProperties: false
}

const topicObject = {
    type: 'object',
    properties: {
        tid: { type: 'number' },
        cid: { type: 'string' },
        mainPid: { type: 'number' },
        topicContent: { type: 'string' },
        postCount: { type: 'number' },
        lastPostTime: { type: 'string' },
        user: userObject,
    },
    additionalProperties: false
}


const getTopics = {
    body: {
        type: 'object',
        required: ['cids', 'page'],
        properties: {
            cids: { type: 'array', items: { type: 'string' } },
            page: { type: 'number' },
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
    getTopics
}