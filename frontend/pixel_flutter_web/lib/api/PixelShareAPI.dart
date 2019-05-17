import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' show Client;

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/models/Category.dart';
import 'package:pixel_flutter_web/models/Post.dart';
import 'package:pixel_flutter_web/models/Topic.dart';
import 'package:pixel_flutter_web/models/User.dart';
import 'package:pixel_flutter_web/models/TopicWithPost.dart';

class PixelShareAPI extends env {
  final Client _http = Client();

  Future<void> register(String username, String password, String email) async {
    await _http.post(url + 'user/register',
        headers: {"Content-Type": "application/json"},
        body: json.encode(
            {'username': username, 'password': password, 'email': email}));
  }

  Future<User> login(String username, String password) async {
    final response = await _http.post(url + 'user/login',
        headers: {"Content-Type": "application/json"},
        body: json.encode({'username': username, 'password': password}));

    final data = json.decode(response.body);
    return User(
        id: data['user']['id'],
        email: data['user']['email'],
        username: data['user']['username'],
        avatarUrl: data['user']['avatar_url'],
        signature: data['user']['signature'],
        token: data['token']);
  }

  Future<List<Category>> getCategories() async {
    final response = await _http.get(url + 'categories/',
        headers: {"Content-Type": "application/json"});
    final data = json.decode(response.body) as List;
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

  Future<List<Topic>> getTopics(int categoryId, int page) async {
    print('getting topics');

    final response = await _http.get(url + 'categories/$categoryId/$page',
        headers: {"Content-Type": "application/json"});
    final data = json.decode(response.body) as List;
    return data.map((rawTopic) {
      return Topic(
          id: rawTopic['id'],
          categoryId: rawTopic['category_id'],
          userId: rawTopic['user']['id'],
          username: rawTopic['user']['username'],
          title: rawTopic['title'],
          body: rawTopic['body'],
          lastReplyTime: rawTopic['last_reply_time'],
          avatarUrl: rawTopic['user']['avatar_url'],
          thumbnail: rawTopic['thumbnail']);
    }).toList();
  }

  Future<TopicWithPost> getTopic(int topicId, int page) async {
    print('getting topic');
    final response = await _http.get(url + 'topic/$topicId/$page',
        headers: {"Content-Type": "application/json"});

    final data = json.decode(response.body);
    final topic = page == 1
        ? Topic(
            id: data['topic']['id'],
            categoryId: data['topic']['category_id'],
            userId: data['topic']['user']['id'],
            username: data['topic']['user']['username'],
            title: data['topic']['title'],
            body: data['topic']['body'],
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
}
