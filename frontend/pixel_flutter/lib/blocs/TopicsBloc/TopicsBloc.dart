import 'package:bloc/bloc.dart';
import 'package:rxdart/rxdart.dart';

import 'package:pixel_flutter/blocs/TopicsBloc/TopicsEvent.dart';
import 'package:pixel_flutter/blocs/TopicsBloc/TopicsState.dart';
import 'package:pixel_flutter/blocs/Repo/TopicsRepo.dart';

class TopicsBloc extends Bloc<TopicsEvent, TopicsState> {
  final topicsRepo = TopicsRepo();

  @override
  Stream<TopicsState> transform(Stream<TopicsEvent> events,
      Stream<TopicsState> Function(TopicsEvent event) next) {
    return super.transform(
      (events as Observable<TopicsEvent>)
          .debounceTime(Duration(milliseconds: 500)),
      next,
    );
  }

  @override
  get initialState => TopicsUninitialized();

  @override
  Stream<TopicsState> mapEventToState(
    TopicsEvent event,
  ) async* {
    if (event is GetTopics && !_hasReachedMax(currentState)) {
      try {
        if (currentState is TopicsUninitialized) {
          final topics = await topicsRepo.getTopics(event.categoryId, 1);
          final maxed = topics.length < 20 ? true : false;

          if (topics.length == 0) {
            yield TopicsNone();
          } else {
            yield TopicsLoaded(topics: topics, hasReachedMax: maxed);
          }
          return;
        }
        if (currentState is TopicsLoaded) {
          final page =
              1 + ((currentState as TopicsLoaded).topics.length / 20).floor();
          final topics = await topicsRepo.getTopics(event.categoryId, page);
          yield topics.length < 20
              ? (currentState as TopicsLoaded).copyWith(hasReachedMax: true)
              : TopicsLoaded(
                  topics: (currentState as TopicsLoaded).topics + topics,
                  hasReachedMax: false);
        }
      } catch (_) {
        yield TopicsError();
      }
    }
  }

  // state handle for last page
  bool _hasReachedMax(TopicsState state) =>
      state is TopicsLoaded && state.hasReachedMax;
}
