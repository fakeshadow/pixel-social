import 'package:bloc/bloc.dart';
import 'package:rxdart/rxdart.dart';

import 'package:pixel_flutter_web/blocs/TopicBloc/TopicEvent.dart';
import 'package:pixel_flutter_web/blocs/TopicBloc/TopicState.dart';
import 'package:pixel_flutter_web/blocs/Repo/TopicsRepo.dart';

class TopicBloc extends Bloc<TopicEvent, TopicState> {
  final topicsRepo = TopicsRepo();

  @override
  Stream<TopicState> transform(Stream<TopicEvent> events,
      Stream<TopicState> Function(TopicEvent event) next) {
    return super.transform(
      (events as Observable<TopicEvent>)
          .debounceTime(Duration(milliseconds: 500)),
      next,
    );
  }

  @override
  get initialState => TopicUninitialized();

  @override
  Stream<TopicState> mapEventToState(
    TopicEvent event,
  ) async* {
    if (event is GetTopic && !_hasReachedMax(currentState)) {
      try {
        if (currentState is TopicUninitialized) {
          final topicWithPost = await topicsRepo.getTopic(event.topicId, 1);
          final maxed = topicWithPost.posts.length < 20 ? true : false;
          yield TopicLoaded(
              topic: topicWithPost.topic,
              posts: topicWithPost.posts,
              hasReachedMax: maxed);
          return;
        }
        if (currentState is TopicLoaded) {
          final page =
              1 + ((currentState as TopicLoaded).posts.length / 20).floor();
          final topicWithPost = await topicsRepo.getTopic(event.topicId, page);
          yield topicWithPost.posts.length < 20
              ? (currentState as TopicLoaded).copyWith(hasReachedMax: true)
              : TopicLoaded(
                  topic: (currentState as TopicLoaded).topic,
                  posts:
                      (currentState as TopicLoaded).posts + topicWithPost.posts,
                  hasReachedMax: false);
        }
      } catch (_) {
        yield TopicError();
      }
    }
  }

  // state handle for last page
  bool _hasReachedMax(TopicState state) =>
      state is TopicLoaded && state.hasReachedMax;
}
