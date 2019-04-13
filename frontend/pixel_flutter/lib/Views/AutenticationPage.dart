import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/InputBlocs.dart';


class AuthenticationPage extends StatefulWidget {
  @override
  _AuthenticationPageState createState() => _AuthenticationPageState();
}

class _AuthenticationPageState extends State<AuthenticationPage> {
  InputBloc inputBloc;

  final TextEditingController _emailController = TextEditingController();
  final TextEditingController _usernameController = TextEditingController();
  final TextEditingController _passwordController = TextEditingController();

  @override
  void initState() {
    super.initState();
    inputBloc = BlocProvider.of<InputBloc>(context);
    _usernameController.addListener(_onUsernameChanged);
    _emailController.addListener(_onEmailChanged);
    _passwordController.addListener(_onPasswordChanged);
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: inputBloc,
        builder: (BuildContext context, InputState state) {
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
                        return state.isUsernameValid ? null : 'Invalid Username';
                      },
                    ),
                    TextFormField(
                      controller: _emailController,
                      decoration: InputDecoration(
                        icon: Icon(Icons.email),
                        labelText: 'Email',
                      ),
                      autovalidate: true,
                      validator: (_) {
                        return state.isEmailValid ? null : 'Invalid Email';
                      },
                    ),
                    TextFormField(
                      controller: _passwordController,
                      decoration: InputDecoration(
                        icon: Icon(Icons.lock),
                        labelText: 'Password',
                      ),
                      obscureText: true,
                      autovalidate: true,
                      validator: (_) {
                        return state.isPasswordValid ? null : 'Invalid Password';
                      },
                    ),
                    RaisedButton(
                      onPressed: state.isFormValid ? _onSubmitPressed : null,
                      child: Text('Submit'),
                    ),
                  ],
                ),
              ));
        });
  }

  @override
  void dispose() {
    _emailController.dispose();
    _passwordController.dispose();
    inputBloc.dispose();
    super.dispose();
  }

  void _onUsernameChanged() {
    inputBloc.dispatch(UsernameChanged(username: _usernameController.text));
  }

  void _onEmailChanged() {
    inputBloc.dispatch(EmailChanged(email: _emailController.text));
  }

  void _onPasswordChanged() {
    inputBloc.dispatch(PasswordChanged(password: _passwordController.text));
  }

  void _onSubmitPressed() {
    inputBloc.dispatch(FormSubmitted());
  }
}