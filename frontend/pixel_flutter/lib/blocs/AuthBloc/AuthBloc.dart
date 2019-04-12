import 'dart:async';
import 'package:bloc/bloc.dart';
import 'package:pixel_flutter/blocs/AuthBlocs.dart';

class AuthBloc extends Bloc<AuthEvent, AuthState> {
  @override
  AuthState get initialState => AuthUninitialized();

  @override
  Stream<AuthState> mapEventToState(
    AuthEvent event,
  ) async* {
    if (event is LoggedIn) {
      yield AuthAuthenticated();
      print(currentState);
    }
//    if (event is LoggedOut) {
//      yield AuthUnauthenticated();
//    }
  }
}
