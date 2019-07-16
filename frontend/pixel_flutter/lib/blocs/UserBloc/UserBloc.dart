import 'package:bloc/bloc.dart';
import 'package:meta/meta.dart';

import 'package:sqflite/sqlite_api.dart';

import 'package:pixel_flutter/blocs/TalkBloc/TalkBloc.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkEvent.dart';
import 'package:pixel_flutter/blocs/ErrorBloc/ErrorBloc.dart';
import 'package:pixel_flutter/blocs/ErrorBloc/ErrorEvent.dart';
import 'package:pixel_flutter/blocs/UserBloc/UserState.dart';
import 'package:pixel_flutter/blocs/UserBloc/UserEvent.dart';
import 'package:pixel_flutter/blocs/Repo/UserRepo.dart';

class UserBloc extends Bloc<UserEvent, UserState> {
  final TalkBloc talkBloc;
  final ErrorBloc errorBloc;
  final Database db;

  UserBloc({this.talkBloc, this.errorBloc, @required this.db});

  UserState get initialState => UserNone();

  @override
  Stream<UserState> mapEventToState(UserEvent event) async* {
    if (event is UserInit) {
      yield Loading();
      final user = await UserRepo.getSelf(db: db)
          .catchError((e) => errorBloc.dispatch(GetError(error: e)));
      if (user.token != null && user != null) {
        talkBloc.dispatch(TalkInit(token: user.token));
        yield UserLoaded(user: user);
      } else if (user.username != null && user.token == null) {
        yield UserLoggedOut(username: user.username);
      } else {
        yield UserNone();
      }
    }

    if (event is Registering) {
      yield Loading();
      try {
        final user = await UserRepo.register(
            username: event.username,
            password: event.password,
            email: event.email,
            db: db);
        talkBloc.dispatch(TalkInit(token: user.token));
        yield UserLoaded(user: user);
      } catch (e) {
        yield Failure(error: e.toString());
      }
    }

    if (event is LoggingIn) {
      yield Loading();
      try {
        final user = await UserRepo.login(
            username: event.username, password: event.password, db: db);
        talkBloc.dispatch(TalkInit(token: user.token));
        yield UserLoaded(user: user);
      } catch (e) {
        yield Failure(error: e.toString());
      }
    }

    if (event is LoggingOut) {
      yield Loading();
      try {
        final username = await UserRepo.deleteToken(db: db);
        talkBloc.dispatch(TalkClose());
        yield UserLoggedOut(username: username);
      } catch (e) {
        yield Failure(error: e.toString());
      }
    }

    //ToDo: delete user does not close websocket connection.
    if (event is Delete) {
      yield Loading();
      UserRepo.deleteUser(db: this.db);
    }
  }
}
