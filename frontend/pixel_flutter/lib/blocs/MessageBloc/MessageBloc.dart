import 'package:bloc/bloc.dart';
import 'package:pixel_flutter/blocs/ErrorBloc/ErrorEvent.dart';
import 'package:rxdart/rxdart.dart';

import 'package:pixel_flutter/blocs/Repo/MessageRepo.dart';

import 'package:pixel_flutter/blocs/ErrorBloc/ErrorBloc.dart';
import 'package:pixel_flutter/blocs/MessageBloc/MessageEvent.dart';
import 'package:pixel_flutter/blocs/MessageBloc/MessageState.dart';

import '../../env.dart';

class MessageBloc extends Bloc<MessageEvent, MessageState> with env {
  final ErrorBloc errorBloc;
  final MessageRepo messageRepo = MessageRepo();

  MessageBloc({this.errorBloc});

  @override
  Stream<MessageState> transform(Stream<MessageEvent> events,
      Stream<MessageState> Function(MessageEvent event) next) {
    return super.transform(
      (events as Observable<MessageEvent>)
          .debounceTime(Duration(milliseconds: 500)),
      next,
    );
  }

  @override
  MessageState get initialState => MessageUninitialized();

  @override
  Stream<MessageState> mapEventToState(
    MessageEvent event,
  ) async* {
    if (event is GotNew) {
      try {
        if (currentState is MessageLoaded) {
          yield MessageLoaded(
              msg: (currentState as MessageLoaded).msg + event.msg);
          return;
        }
      } catch (e) {
        errorBloc.dispatch(GetError(error: e.toString()));
      }
    }
    if (event is GotHistory) {
      try {
        if (currentState is MessageUninitialized) {
          yield MessageLoaded(msg: event.msg);
          return;
        }
        if (currentState is MessageLoaded) {
          yield MessageLoaded(
              msg: event.msg + (currentState as MessageLoaded).msg);
          return;
        }
      } catch (e) {
        errorBloc.dispatch(GetError(error: e.toString()));
      }
    }
  }
}
