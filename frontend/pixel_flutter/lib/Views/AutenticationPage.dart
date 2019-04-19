import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/RegisterBlocs.dart';
import 'package:pixel_flutter/blocs/UserBlocs.dart';

/// pass in type and username for login form and type only for register
class AuthenticationPage extends StatefulWidget {
  final String type, username;

  AuthenticationPage({@required this.type, this.username});

  @override
  _AuthenticationPageState createState() => _AuthenticationPageState();
}

class _AuthenticationPageState extends State<AuthenticationPage> {
  RegisterBloc _registerBloc;
  UserBloc _userBloc;
  String _type;

  final TextEditingController _emailController = TextEditingController();
  final TextEditingController _usernameController = TextEditingController();
  final TextEditingController _passwordController = TextEditingController();

  @override
  void initState() {
    _type = widget.type;
    if (_type == 'login') {
      _usernameController.text = widget.username;
    }
    _registerBloc = RegisterBloc();
    _userBloc = BlocProvider.of<UserBloc>(context);
    _usernameController.addListener(_onUsernameChanged);
    _emailController.addListener(_onEmailChanged);
    _passwordController.addListener(_onPasswordChanged);
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return BlocListener(
      bloc: _userBloc,
      listener: (context, state) {
        if (state is UserLoaded) {
          Navigator.pop(context);
        }
      },
      child: BlocBuilder(
          bloc: _registerBloc,
          builder: (BuildContext context, RegisterState state) {
            // need to find a better way to handle login dispatch
            _registerBloc
                .dispatch(UsernameChanged(username: _usernameController.text));
            return Scaffold(
                body: Form(
              child: Column(
                children: <Widget>[
                  TextFormField(
                    controller: _usernameController,
                    decoration: InputDecoration(
                      icon: Icon(Icons.person),
                      labelText: 'Username',
                    ),
                    autovalidate: true,
                    validator: (_) {
                      return state.isUsernameValid || state.username.length < 1
                          ? null
                          : 'Invalid Username';
                    },
                  ),
                  _type == 'register'
                      ? TextFormField(
                          controller: _emailController,
                          decoration: InputDecoration(
                            icon: Icon(Icons.email),
                            labelText: 'Email',
                          ),
                          autovalidate: true,
                          validator: (_) {
                            return state.isEmailValid || state.email.length < 1
                                ? null
                                : 'Invalid Email';
                          },
                        )
                      : Container(),
                  TextFormField(
                    controller: _passwordController,
                    decoration: InputDecoration(
                      icon: Icon(Icons.lock),
                      labelText: 'Password',
                    ),
                    obscureText: true,
                    autovalidate: true,
                    validator: (_) {
                      return state.isPasswordValid || state.password.length < 1
                          ? null
                          : 'Invalid Password';
                    },
                  ),
                  RaisedButton(
                    onPressed: state.isRegisterValid && _type == 'register'
                        ? () => _submit(state)
                        : state.isLoginValid && _type == 'login'
                            ? () => _submit(state)
                            : null,
                    child: Text('Submit'),
                  ),
                ],
              ),
            ));
          }),
    );
  }

  @override
  void dispose() {
    _emailController.dispose();
    _passwordController.dispose();
    _registerBloc.dispose();
    _userBloc.dispose();
    super.dispose();
  }

  void _onUsernameChanged() {
    _registerBloc.dispatch(UsernameChanged(username: _usernameController.text));
  }

  void _onEmailChanged() {
    _registerBloc.dispatch(EmailChanged(email: _emailController.text));
  }

  void _onPasswordChanged() {
    _registerBloc.dispatch(PasswordChanged(password: _passwordController.text));
  }

  void _submit(RegisterState state) {
    if (_type == 'register') {
      _userBloc.dispatch(Registering(
          username: state.username,
          password: state.password,
          email: state.email));
      _registerBloc.dispatch(FormReset());
    } else {
      _userBloc.dispatch(
          LoggingIn(username: state.username, password: state.password));
    }
  }
}
