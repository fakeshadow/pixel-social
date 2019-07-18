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
import 'package:pixel_flutter/blocs/UsersBloc/UsersBloc.dart';
import 'package:pixel_flutter/blocs/UsersBloc/UsersEvent.dart';

import 'package:pixel_flutter/blocs/Repo/TalkRepo.dart';

import 'package:pixel_flutter/models/Talk.dart';
import 'package:pixel_flutter/models/Message.dart';
import 'package:pixel_flutter/models/User.dart';

import 'package:pixel_flutter/env.dart';

class TalkBloc extends Bloc<TalkEvent, TalkState> with env {
  final MessageBloc messageBloc;
  final UsersBloc usersBloc;
  final ErrorBloc errorBloc;
  final Database db;

  TalkBloc({this.usersBloc, this.messageBloc, this.errorBloc, this.db});

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
        // add listener for websocket and use local jwt token for websocket authentication
        talkRepo.addListener(handleMessage);
        talkRepo.init(token: event.token);

        // get talks,friends and unread messages.
        final talks = await talkRepo.getTalks(db: db);
        final users = await talkRepo.getRelation(db: db);
//        talkRepo.getUpdate();

        usersBloc.dispatch(GotUsers(users: users));
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
          talkRepo.setTalks(talks: event.talks, db: db);

          yield TalkLoaded(talks: event.talks + talksOld);
        } else {
          talkRepo.setTalks(talks: event.talks, db: db);
          yield TalkLoaded(talks: event.talks);
        }
      } catch (e) {
        errorBloc.dispatch(GetError(error: e.toString()));
      }
    }
  }

  Future<void> handleMessage(String msg) async {
    print(msg);
    if (msg.startsWith('!')) {
      return;
    } else if (msg.startsWith('!!!')) {
      final str = msg.substring(4);
      if (str.length == 0) {
        return;
      }
      errorBloc.dispatch(GetError(error: str));
      return;
    } else if (msg.startsWith('/')) {
      final index = msg.indexOf(" ");
      final cmd = msg.substring(0, index);
      final str = msg.substring(index);
      if (str.length == 0) {
        return;
      }
      switch (cmd) {
        case '/msg':
          gotMsg(str);
          break;
        case "/talks":
          gotTalks(str);
          break;
        case "/users":
          gotUsers(str);
          break;
        case "/relation":
          gotUsers(str);
          break;
      }
      return;
    }
    final data = jsonDecode(msg);
    switch (data['typ']) {
      case "relation":
        gotRelation(data);
        break;
      case 'message':
        gotMsg(data);
        break;
      case "talks":
        gotTalks(data);
        break;
      case "users":
        gotUsers(data);
        break;
    }
  }

  Future<void> gotUsers(String msg) {
    final data = jsonDecode(msg) as List;
    final result = data.map((rawUser) {
      return User(
          id: rawUser['id'],
          username: rawUser['username'],
          email: rawUser['email'],
          avatarUrl: rawUser['avatarUrl'],
          signature: rawUser['signature']);
    }).toList();

    usersBloc.dispatch(GotUsers(users: result));
    return null;
  }

  Future<void> gotTalks(String msg) {
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

    dispatch(GotTalks(talks: result));
    return null;
  }

  Future<void> gotRelation(Map map) {
    final friends = map['friends'].cast<int>();
    usersBloc.dispatch(GotRelations(friends: friends));
    return null;
  }

  Future<void> gotMsg(String msg) {
    final data = jsonDecode(msg) as List;
    final result = data.map((msg) {
      // ToDo: use substring to handle date time as flutter doesn't support parsing nanoseconds.
      final String time = (msg['time'] as String).substring(0, 26);
      return Message(
          talkId: msg['talk_id'],
          userId: msg['user_id'],
          dateTime: DateTime.parse(time),
          msg: msg['msg']);
    }).toList();

    messageBloc.dispatch(GotHistory(msg: result));
    //ToDo: update talk last reply and unread reply count.
    return null;
  }
}
