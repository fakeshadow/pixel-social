import 'package:bloc/bloc.dart';
import 'package:meta/meta.dart';

import 'package:sqflite/sqlite_api.dart';

import 'package:pixel_flutter/blocs/ErrorBloc/ErrorBloc.dart';
import 'package:pixel_flutter/blocs/ErrorBloc/ErrorEvent.dart';
import 'package:pixel_flutter/blocs/UsersBloc/UsersState.dart';
import 'package:pixel_flutter/blocs/UsersBloc/UsersEvent.dart';

import 'package:pixel_flutter/api/DataBase.dart';

class UsersBloc extends Bloc<UsersEvent, UsersState> {
  final ErrorBloc errorBloc;
  final Database db;

  UsersBloc({this.errorBloc, @required this.db});

  UsersState get initialState => UsersState.initial();

  @override
  Stream<UsersState> mapEventToState(UsersEvent event) async* {
    if (event is GotRelations) {
      event.friends.removeWhere((uid) {
        return currentState.friends.contains(uid);
      });

      yield currentState.copyWith(
          friends: currentState.friends + event.friends);
    }

    if (event is GotUsers) {
      DataBase.setUsers(db: db, users: event.users)
          .catchError((e) => errorBloc.dispatch(GetError(error: e)));
      yield currentState.copyWith(users: currentState.users + event.users);
    }
  }
}
