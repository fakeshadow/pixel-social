import 'dart:convert';

import 'package:sqflite/sqlite_api.dart';
import 'package:bloc/bloc.dart';
import 'package:rxdart/rxdart.dart';

import 'package:pixel_flutter/blocs/ErrorBloc/ErrorBloc.dart';
import 'package:pixel_flutter/blocs/ErrorBloc/ErrorEvent.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkEvent.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkState.dart';

import 'package:pixel_flutter/blocs/Repo/TalkRepo.dart';

import 'package:pixel_flutter/models/Talk.dart';

import 'package:pixel_flutter/env.dart';


class TalkBloc extends Bloc<TalkEvent, TalkState> with env {
  final ErrorBloc errorBloc;
  final Database db;

  TalkBloc({this.errorBloc, this.db});

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
        final talks = await talkRepo.getTalks(db: db);
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
      final str = msg.split(" ").toList();
      if (str.length != 2) {
        return null;
      }
      if (str[0] == "/talks") {
        final data = jsonDecode(str[1]) as List;
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

        talkRepo.saveTalks(talks: result);

        dispatch(GotTalks(talks: result));
      }
    }
    return null;
  }
}
