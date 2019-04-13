import 'package:bloc/bloc.dart';
import 'package:rxdart/rxdart.dart';

import 'package:pixel_flutter/blocs/TopicBloc/TopicEvent.dart';
import 'package:pixel_flutter/blocs/TopicBloc/TopicState.dart';
import 'package:pixel_flutter/blocs/Repo/TopicRepo.dart';

class TopicBloc extends Bloc<TopicEvent, TopicState> {
  final topicRepo = TopicRepo();

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
    if (event is GetTopics && !_hasReachedMax(currentState)) {
      try {
        if (currentState is TopicUninitialized) {
          final topics = await topicRepo.getTopics(event.categoryId, 1);
          yield TopicLoaded(topics: topics, hasReachedMax: false);
          return;
        }
        if (currentState is TopicLoaded) {
          final page =
              ((currentState as TopicLoaded).topics.length / 20).ceil();
          final topics = await topicRepo.getTopics(event.categoryId, page);
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



  // state handle for last page
  bool _hasReachedMax(TopicState state) =>
      state is TopicLoaded && state.hasReachedMax;
}
