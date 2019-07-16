import 'dart:convert';

import 'package:sqflite/sqlite_api.dart';
import 'package:bloc/bloc.dart';
import 'package:rxdart/rxdart.dart';

import 'package:pixel_flutter/blocs/ErrorBloc/ErrorBloc.dart';
import 'package:pixel_flutter/blocs/ErrorBloc/ErrorEvent.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkEvent.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkState.dart';
import 'package:pixel_flutter/blocs/MessageBloc/MessageBloc.dart';
import 'package:pixel_flutter/blocs/MessageBloc/MessageEvent.dart';

import 'package:pixel_flutter/blocs/Repo/TalkRepo.dart';

import 'package:pixel_flutter/models/Talk.dart';
import 'package:pixel_flutter/models/Message.dart';

import 'package:pixel_flutter/env.dart';

class TalkBloc extends Bloc<TalkEvent, TalkState> with env {
  final MessageBloc messageBloc;
  final ErrorBloc errorBloc;
  final Database db;

  TalkBloc({this.messageBloc, this.errorBloc, this.db});

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
    if (event is TalkInit) {
      try {
        talkRepo.addListener(handleMessage);
        talkRepo.init(token: event.token);
        final talks = await talkRepo.getTalksLocal(db: db);
        yield TalkLoaded(talks: talks);
      } catch (e) {
        errorBloc.dispatch(GetError(error: e.toString()));
      }
    }
    if (event is TalkClose) {
      talkRepo.close();
      talkRepo.removeListener(handleMessage);
      yield TalkUninitialized();
    }
    if (event is SendMessage) {
      talkRepo.sendMessage(event.msg);
      yield currentState;
    }
    if (event is GotTalks) {
      try {
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
        } else {
          yield TalkLoaded(talks: event.talks);
        }
      } catch (e) {
        errorBloc.dispatch(GetError(error: e.toString()));
      }
    }
  }

  Future<void> handleMessage(String msg) async {
    print(msg);
    if (msg.startsWith('!!!')) {
      final str = msg.substring(4);
      if (str.length == 0) {
        return null;
      }
      errorBloc.dispatch(GetError(error: str));
    }
    if (msg.startsWith('/')) {
      final index = msg.indexOf(" ");
      final cmd = msg.substring(0, index);
      final str = msg.substring(index);
      if (str.length == 0) {
        return null;
      }
      switch (cmd) {
        case '/msg':
          gotMsg(str);
          break;
        case "/talks":
          gotTalks(str);
          break;
      }
    }
  }

  void gotTalks(String msg) {
    final data = jsonDecode(msg) as List;
    final result = data.map((rawTalk) {
      return Talk(
          id: rawTalk['id'],
          name: rawTalk['name'],
          description: rawTalk['description'],
          privacy: rawTalk['privacy'],
          owner: rawTalk['owner'],
          admin: rawTalk['admin'].cast<int>(),
          users: rawTalk['users'].cast<int>());
    }).toList();

    talkRepo.setTalks(talks: result, db: db);
    dispatch(GotTalks(talks: result));
  }

  void gotMsg(String msg) {
    final data = jsonDecode(msg) as List;
    final result = data.map((msg) {
      // ToDo: use substring to handle date time as flutter doesn't support nano seconds date time for now.
      final String time = (msg['time'] as String).substring(0, 26);
      return Message(
          talkId: msg['talk_id'],
          userId: msg['user_id'],
          dateTime: DateTime.parse(time),
          msg: msg['msg']);
    }).toList();

    messageBloc.dispatch(GotMessage(msg: result));
    //ToDo: update talk last reply and unread reply count.
  }
}
