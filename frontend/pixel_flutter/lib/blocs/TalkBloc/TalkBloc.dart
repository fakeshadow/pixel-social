import 'package:bloc/bloc.dart';
import 'package:rxdart/rxdart.dart';

import 'package:pixel_flutter/blocs/TalkBloc/TalkEvent.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkState.dart';

import '../../env.dart';

class TalkBloc extends Bloc<TalkEvent, TalkState> with env {
  @override
  Stream<TalkState> transform(Stream<TalkEvent> events,
      Stream<TalkState> Function(TalkEvent event) next) {
    return super.transform(
      (events as Observable<TalkEvent>)
          .debounceTime(Duration(milliseconds: 500)),
      next,
    );
  }

  @override
  TalkState get initialState => TalkUninitialized();

  @override
  Stream<TalkState> mapEventToState(
    TalkEvent event,
  ) async* {
    if (event is GotTalk) {
      try {
        if (currentState is TalkUninitialized) {
          yield TalkLoaded(talks: event.talks);
          return;
        }
        if (currentState is TalkLoaded) {
          final talksOld = (currentState as TalkLoaded).talks.where((t) {
            var result = true;
            for (var tt in event.talks) {
              if (t.id == tt.id) {
                result = false;
                break;
              }
            }
            return result;
          }).toList();
          yield TalkLoaded(talks: event.talks + talksOld);
          return;
        }
      } catch (_) {
        yield TalkError(error: "error");
      }
    }
  }
}
