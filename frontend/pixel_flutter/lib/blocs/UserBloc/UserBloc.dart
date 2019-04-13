import 'dart:async';
import 'package:bloc/bloc.dart';
import 'package:flutter/widgets.dart';

import 'package:pixel_flutter/blocs/InputBlocs.dart';
import 'package:pixel_flutter/blocs/UserBlocs.dart';
import 'package:pixel_flutter/blocs/Repo/UserRepo.dart';

class UserBloc extends Bloc<UserEvent, UserState> {
  final userRepo = UserRepo();
  final InputBloc inputBloc;

  StreamSubscription inputSubscription;

  @override
  void dispose() {
    inputSubscription.cancel();
    super.dispose();
  }

  UserBloc({@required this.inputBloc}) {
    inputSubscription = inputBloc.state.listen((state) {
      if (state.formSubmittedSuccessfully == true) {
        dispatch(Registering(
          username: state.username,
          password: state.password,
          email: state.email,
        ));
      }
    });
  }

  UserState get initialState => AppStarted();

  @override
  Stream<UserState> mapEventToState(UserEvent event) async* {
    if (event is UserInit) {
      yield Loading();
      final hasToken = await userRepo.hasToken();
      if (hasToken) {
        final user = await userRepo.getLocalUser();
        yield UserLoaded(user: user);
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
        );
        yield UserLoaded(user: user);
      } catch (e) {
        yield Failure(error: e.toString());
      }
    }

    if (event is LoggingIn) {
      yield Loading();
      try {
        final user = await userRepo.login(
          username: event.username,
          password: event.password,
        );
        await userRepo.saveUser(user);
        yield UserLoaded(user: user);
      } catch (e) {
        yield Failure(error: e.toString());
      }
    }

    if (event is LoggingOut) {
      yield Loading();
      try {
        await userRepo.deleteToken();
        yield UserNone();
      } catch (e) {
        yield Failure(error: e.toString());
      }
    }
  }
}
