import 'package:bloc/bloc.dart';
import 'package:rxdart/rxdart.dart';

import 'package:pixel_flutter/blocs/Repo/MessageRepo.dart';

import 'package:pixel_flutter/blocs/MessageBloc/MessageEvent.dart';
import 'package:pixel_flutter/blocs/MessageBloc/MessageState.dart';

import '../../env.dart';

class MessageBloc extends Bloc<MessageEvent, MessageState> with env {
  final MessageRepo messageRepo = MessageRepo();

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
    if (event is GotPublicMessage) {
      try {
        if (currentState is MessageUninitialized) {
          yield MessageLoaded(pubMsg: event.msg);
          return;
        }
        if (currentState is MessageLoaded) {
          yield MessageLoaded(
              pubMsg: (currentState as MessageLoaded).pubMsg + event.msg);
          return;
        }
      } catch (_) {
        yield MessageError(error: "error");
      }
    }
    if (event is GotPrivateMessage) {
      try {
        if (currentState is MessageUninitialized) {
          yield MessageLoaded(prvMsg: event.msg);
          return;
        }
        if (currentState is MessageLoaded) {
          yield MessageLoaded(
              prvMsg: (currentState as MessageLoaded).prvMsg + event.msg);
          return;
        }
      } catch (_) {
        yield MessageError(error: "error");
      }
    }
  }
}
