import 'package:bloc/bloc.dart';
import 'package:sqflite/sqlite_api.dart';

import 'package:pixel_flutter/blocs/TalkBloc/TalkBloc.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkEvent.dart';
import 'package:pixel_flutter/blocs/UserBlocs.dart';
import 'package:pixel_flutter/blocs/Repo/UserRepo.dart';

class UserBloc extends Bloc<UserEvent, UserState> {
  final userRepo = UserRepo();
  final TalkBloc talkBloc;
  final Database db;

  UserBloc({this.talkBloc, this.db});

  UserState get initialState => UserNone();

  @override
  Stream<UserState> mapEventToState(UserEvent event) async* {
    if (event is UserInit) {
      yield Loading();
      final user = event.user;
      if (user.token != null && user != null) {
        talkBloc.dispatch(TalkInit(token: user.token));
        yield UserLoaded(user: user);
      } else if (user != null) {
        yield UserLoggedOut(username: user.username);
      } else {
        yield UserNone();
      }
    }

    if (event is Registering) {
      yield Loading();
      try {
        final user = await userRepo.register(
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
        final user = await userRepo.login(
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
        final username = await userRepo.deleteToken(db: db);
        talkBloc.dispatch(TalkClose());
        yield UserLoggedOut(username: username);
      } catch (e) {
        yield Failure(error: e.toString());
      }
    }

    if (event is Delete) {
      yield Loading();
      await userRepo.deleteUser(db: this.db);
    }
  }
}
