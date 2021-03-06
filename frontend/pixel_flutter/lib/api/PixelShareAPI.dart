import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' show Client;

import 'package:pixel_flutter/env.dart';
import 'package:pixel_flutter/models/Category.dart';
import 'package:pixel_flutter/models/Post.dart';
import 'package:pixel_flutter/models/Topic.dart';
import 'package:pixel_flutter/models/User.dart';
import 'package:pixel_flutter/models/TopicWithPost.dart';

class PixelShareAPI {
  static final Client _http = Client();

  static Future<void> register(
      String username, String password, String email) async {
    final response = await _http.post(env.url + 'auth/register',
        headers: {"Content-Type": "application/json"},
        body: json.encode(
            {'username': username, 'password': password, 'email': email}));
    final data = json.decode(response.body);
    if (data['error'] != null) {
      return Future.error(data['error']);
    }
  }

  static Future<User> login(String username, String password) async {
    final response = await _http.post(env.url + 'auth/login',
        headers: {"Content-Type": "application/json"},
        body: json.encode({'username': username, 'password': password}));

    final data = json.decode(response.body);

    if (data['error'] != null) {
      return Future.error(data['error']);
    }

    return User(
        id: data['user']['id'],
        privilege: data['user']['privilege'],
        email: data['user']['email'],
        username: data['user']['username'],
        avatarUrl: data['user']['avatar_url'],
        signature: data['user']['signature'],
        token: data['token']);
  }

  static Future<List<Category>> getCategories() async {
    final response = await _http.get(env.url + 'categories?query_type=All',
        headers: {"Content-Type": "application/json"});
    final data = json.decode(response.body) as List;

    if (data.isNotEmpty && data.first['error'] != null) {
      return Future.error(data.first['error']);
    }

    return data.map((rawCategories) {
      return Category(
          id: rawCategories['id'],
          name: rawCategories['name'],
          thumbnail: rawCategories['thumbnail'],
          postCount: rawCategories['post_count'],
          subCount: rawCategories['subscriber_count'],
          topicCount: rawCategories['topic_count']);
    }).toList();
  }

  static Future<List<Topic>> getTopics(int categoryId, int page) async {
    final response = await _http.get(env.url + 'categories?query_type=Popular&category_id=$categoryId&page=$page',
        headers: {"Content-Type": "application/json"});
    final data = json.decode(response.body) as List;

    if (data.isNotEmpty && data.first['error'] != null) {
      return Future.error(data.first['error']);
    }

    return data.map((rawTopic) {
      return Topic(
          id: rawTopic['id'],
          categoryId: rawTopic['category_id'],
          userId: rawTopic['user']['id'],
          username: rawTopic['user']['username'],
          title: rawTopic['title'],
          body: rawTopic['body'],
          replyCount: rawTopic['reply_count'],
          lastReplyTime: rawTopic['last_reply_time'],
          avatarUrl: rawTopic['user']['avatar_url'],
          thumbnail: rawTopic['thumbnail']);
    }).toList();
  }

  static Future<TopicWithPost> getTopic(int topicId, int page) async {
    final response = await _http.get(env.url + 'topic/$topicId/$page',
        headers: {"Content-Type": "application/json"});

    final data = json.decode(response.body);

    if (data['error'] != null) {
      return Future.error(data['error']);
    }

    final topic = page == 1
        ? Topic(
            id: data['topic']['id'],
            categoryId: data['topic']['category_id'],
            userId: data['topic']['user']['id'],
            username: data['topic']['user']['username'],
            title: data['topic']['title'],
            body: data['topic']['body'],
            replyCount: data['reply_count'],
            lastReplyTime: data['topic']['last_reply_time'],
            avatarUrl: data['topic']['user']['avatar_url'],
            thumbnail: data['topic']['thumbnail'])
        : null;

    final posts = data['posts'].map((rawPost) {
      return Post(
          id: rawPost['id'],
          userId: rawPost['user']['id'],
          username: rawPost['user']['username'],
          avatarUrl: rawPost['user']['avatar_url'],
          topicId: rawPost['topic_id'],
          postId: rawPost['post_id'],
          postContent: rawPost['post_content'],
          lastReplyTime: rawPost['last_reply_time'],
          replyCount: rawPost['reply_count']);
    }).toList();

    return TopicWithPost(topic: topic, posts: posts);
  }

  static Future<Topic> addTopic(Topic topic, String jwt) async {
    final response = await _http.post(env.url + 'topic',
        headers: {
          "Content-Type": "application/json",
          "AUTHORIZATION": "Bearer " + jwt
        },
        body: json.encode({
          'title': topic.title,
          'body': topic.body,
          'thumbnail': topic.thumbnail,
          'category_id': topic.categoryId
        }));
    final data = json.decode(response.body);

    if (data['error'] != null) {
      return Future.error(data['error']);
    }

    return Topic(
        id: data['id'],
        categoryId: data['category_id'],
        userId: data['user_id'],
        title: data['title'],
        body: data['body'],
        replyCount: data['reply_count'],
        lastReplyTime: data['last_reply_time'],
        thumbnail: data['thumbnail']);
  }
}
