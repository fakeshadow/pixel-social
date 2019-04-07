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
  TopicState get initialState => TopicUninitialized();

  @override
  Stream<TopicState> mapEventToState(
    currentState,
    event,
  ) async* {
    if (event is TopicAPI && !_hasReachedMax(currentState)) {
      try {
        if (currentState is TopicUninitialized) {
          final topics = await _getTopics('9999');
          yield TopicLoaded(topics: topics, hasReachedMax: false);
        }
        if (currentState is TopicLoaded && currentState.topics.length < 20) {
          yield currentState.copyWith(hasReachedMax: true);
        }
        if (currentState is TopicLoaded && currentState.topics.length >= 20) {
          final topics = await _getTopics(currentState.topics[19].lastPostTime);
          yield topics.length < 50
              ? TopicLoaded(
                  topics: currentState.topics + topics, hasReachedMax: true)
              : TopicLoaded(
                  topics: currentState.topics + topics, hasReachedMax: false);
        }
      } catch (_) {
        yield TopicError();
      }
    }
  }

  bool _hasReachedMax(TopicState state) =>
      state is TopicLoaded && state.hasReachedMax;

  Future<List<Topic>> _getTopics(String lastPostTime) async {
    final response =
        await httpClient.post('http://192.168.1.197:3100/api/topic',
            headers: {
              "Authorization":
                  "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1aWQiOjUsImlhdCI6MTU1MDE5MDA2M30.wLwh2W5nezC4F7TcK6iPbJJitFByCQmItWtuTcSHDpc",
              "Content-Type": "application/json"
            },
            body: json.encode({
              "cids": ["2"],
              "lastPostTime": lastPostTime
            }));
    print(response);
    if (response.statusCode == 200) {
      final data = json.decode(response.body) as List;
      return data.map((rawTopic) {
        return Topic(
            tid: rawTopic['tid'],
            cid: rawTopic['cid'],
            mainPid: rawTopic['mainPid'],
            topicContent: rawTopic['topicContent'],
            postCount: rawTopic['postCount'],
            lastPostTime: rawTopic['lastPostTime'],
            username: rawTopic['user']['username'],
            avatar: rawTopic['user']['avatar']
            );
            
      }).toList();
    } else {
      throw Exception('error getting Topics');
    }
  }
}
