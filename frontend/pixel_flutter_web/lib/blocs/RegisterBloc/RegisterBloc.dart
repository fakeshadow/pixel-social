import 'package:bloc/bloc.dart';

import 'package:pixel_flutter_web/blocs/RegisterBlocs.dart';

/// used for both register and login.
class RegisterBloc extends Bloc<RegisterEvent, RegisterState> {
  final RegExp _emailRegExp = RegExp(
    r'^[a-zA-Z0-9.!#$%&’*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$',
  );
  final RegExp _passwordRegExp = RegExp(
    r'^(?=.*[A-Za-z])(?=.*\d)[A-Za-z\d]{8,}$',
  );

  @override
  RegisterState get initialState => RegisterState.initial();

  @override
  Stream<RegisterState> mapEventToState(
      RegisterEvent event,
      ) async* {
    if (event is UsernameChanged) {
      yield currentState.copyWith(
        username: event.username,
        isUsernameValid: _isUsernameValid(event.username),
      );
    }

    if (event is EmailChanged) {
      yield currentState.copyWith(
        email: event.email,
        isEmailValid: _isEmailValid(event.email),
      );
    }
    if (event is PasswordChanged) {
      yield currentState.copyWith(
        password: event.password,
        isPasswordValid: _isPasswordValid(event.password),
      );
    }
    if (event is FormReset) {
      yield RegisterState.initial();
    }
  }

  bool _isUsernameValid(String username) {
    return username.length > 5;
  }

  bool _isEmailValid(String email) {
    return _emailRegExp.hasMatch(email);
  }

  bool _isPasswordValid(String password) {
    return _passwordRegExp.hasMatch(password);
  }
}
