import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' show Client;
import 'package:pixel_flutter/models/Post.dart';
import 'package:pixel_flutter/models/Topic.dart';

class PostsAPI {
  final Client _client = Client();
  static const String _url = 'http://192.168.1.197:3100/api/';
  List<Topic> topics = [];

  Future<List<Topic>> getTopics({
    int page = 1,
  }) async {
    await Future.delayed(const Duration(seconds: 2));
    await _client
        .get(Uri.parse('http://192.168.1.197:3100/api/topic/1'), headers: {
          "Authorization": "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1aWQiOjUsImlhdCI6MTU1MDE5MDA2M30.wLwh2W5nezC4F7TcK6iPbJJitFByCQmItWtuTcSHDpc"
        })
        .then((res) => res.body)
        .then(json.decode)
        .then((json) => json.forEach((topic) {
              Topic tp = Topic.fromJson(topic);
              topics.add(tp);
            }))
        .catchError((err) => print(err));
    return topics;
  }

  Future<List<Post>> getPosts({
    int page = 1,
  }) async {
    List<Post> posts = [];
    await Future.delayed(const Duration(seconds: 2));
    await _client
        .get(Uri.parse(_url + 'topic/$page'))
        .then((res) => res.body)
        .then(json.decode)
        .then((json) => json.forEach((post) {
              Post po = Post.fromJson(post);
              posts.add(po);
            }))
        .catchError((err) => print(err));
    return posts;
  }

  Future<String> postPost({
    String topicId,
    String titleData,
    String postData,
  }) async {
    await _client
        .post(Uri.parse(_url + "topic"), headers: {
          "Content-Type": "application/json"
        }, body: {
          "cid": topicId,
          "titleData": titleData,
          "postData": postData
        })
        .then((res) => print(res))
        .catchError((err) => print(err));
    return 'Success';
  }
}
