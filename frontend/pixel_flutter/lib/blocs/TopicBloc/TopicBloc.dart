import 'package:bloc/bloc.dart';
import 'package:meta/meta.dart';
import 'dart:convert';

import 'package:http/http.dart' as http;
import 'package:pixel_flutter/blocs/Blocs.dart';
import 'package:pixel_flutter/models/Topic.dart';
import 'package:rxdart/rxdart.dart';

class TopicBloc extends Bloc<TopicEvent, TopicState> {
  final http.Client httpClient;

  TopicBloc({@required this.httpClient});

  @override
  Stream<TopicEvent> transform(Stream<TopicEvent> events) {
    return (events as Observable<TopicEvent>)
        .debounce(Duration(milliseconds: 500));
  }

  @override
  get initialState => TopicUninitialized();

  @override
  Stream<TopicState> mapEventToState(
    TopicEvent event,
  ) async* {
    if (event is TopicAPI && !_hasReachedMax(currentState)) {
      try {
        if (currentState is TopicUninitialized) {
          final topics = await _getTopics(1, 1);
          yield TopicLoaded(topics: topics, hasReachedMax: false);
          return;
        }
        if (currentState is TopicLoaded) {
          final topics = await _getTopics(1, 1);
          yield topics.isEmpty
              ? (currentState as TopicLoaded).copyWith(hasReachedMax: true)
              : TopicLoaded(
                  topics: (currentState as TopicLoaded).topics + topics,
                  hasReachedMax: false);
        }
      } catch (_) {
        yield TopicError();
      }
    }
  }

  bool _hasReachedMax(TopicState state) =>
      state is TopicLoaded && state.hasReachedMax;

  Future<List<Topic>> _getTopics(int categoryId, int page) async {
    final response =
        await httpClient.get('http://192.168.1.197:3200/categories/1/1', headers: {
//      "Authorization":
//          "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1aWQiOjUsImlhdCI6MTU1MDE5MDA2M30.wLwh2W5nezC4F7TcK6iPbJJitFByCQmItWtuTcSHDpc",
      "Content-Type": "application/json"
    });

    if (response.statusCode == 200) {
      final data = json.decode(response.body) as List;
      return data.map((rawTopic) {
        return Topic(
            id: rawTopic['id'],
            categoryId: rawTopic['category_id'],
            userId: rawTopic['user']['user_id'],
            username: rawTopic['user']['username'],
            title: rawTopic['title'],
            body: rawTopic['body'],
            lastReplyTime: rawTopic['last_reply_time'],
            avatarUrl: rawTopic['user']['avatar_url'],
            thumbnail: rawTopic['thumbnail']);
      }).toList();
    } else {
      print("????");
      throw Exception('error getting Topics');
    }
  }
}
