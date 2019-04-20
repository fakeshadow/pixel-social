import 'package:flutter/material.dart';
import 'package:flutter/painting.dart';
import 'package:flutter/widgets.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/RegisterBlocs.dart';
import 'package:pixel_flutter/blocs/UserBlocs.dart';
import 'package:pixel_flutter/components/Background/GeneralBackground.dart';
import 'package:pixel_flutter/components/Button/AnimatedSubmitButton.dart';
import 'package:pixel_flutter/components/NavigationBar/AuthenticationNavBar.dart';
import 'package:pixel_flutter/style/text.dart';

/// pass in type and username for login form and type only for register
class AuthenticationPage extends StatefulWidget {
  final String type, username;

  AuthenticationPage({@required this.type, this.username});

  @override
  _AuthenticationPageState createState() => _AuthenticationPageState();
}

class _AuthenticationPageState extends State<AuthenticationPage>
    with SingleTickerProviderStateMixin {
  RegisterBloc _registerBloc;
  UserBloc _userBloc;
  String _type;

  AnimationController _animationController;
  Animation<double> _animationDouble;

  final TextEditingController _emailController = TextEditingController();
  final TextEditingController _usernameController = TextEditingController();
  final TextEditingController _passwordController = TextEditingController();

  initAnimation() {
    _animationController.forward();
  }

  @override
  void initState() {
    _type = widget.type;
    if (_type == 'Login') {
      _usernameController.text = widget.username;
    }
    _registerBloc = RegisterBloc();
    _userBloc = BlocProvider.of<UserBloc>(context);
    _usernameController.addListener(_onUsernameChanged);
    _emailController.addListener(_onEmailChanged);
    _passwordController.addListener(_onPasswordChanged);

    _animationController =
        AnimationController(vsync: this, duration: Duration(milliseconds: 700));
    _animationDouble =
        Tween<double>(begin: 0.0, end: 1.0).animate(_animationController);
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: _registerBloc,
        builder: (BuildContext context, RegisterState state) {
          // need to find a better way to handle login dispatch
          _registerBloc
              .dispatch(UsernameChanged(username: _usernameController.text));
          return Hero(
              tag: 'auth',
              child: Scaffold(
                  body: BlocListener(
                      //ToDo: Change error handling to error bloc
                      bloc: _userBloc,
                      listener: (context, userState) {
                        if (userState is UserLoaded) {
                          Navigator.pop(context);
                        }
                        if (userState is Failure) {
                          Scaffold.of(context).showSnackBar(SnackBar(
                            duration: Duration(seconds: 2),
                            content: Text(userState.error),
                            backgroundColor: Colors.deepOrange,
                          ));
                          _userBloc.dispatch(UserInit());
                        }
                      },
                      child: FutureBuilder(
                        future: initAnimation(),
                        builder: (context, snapshot) => FadeTransition(
                              opacity: _animationDouble,
                              child: Stack(children: <Widget>[
                                GeneralBackground(),
                                SingleChildScrollView(
                                    child: Column(
                                  children: <Widget>[
                                    AuthNavBar(),
                                    Material(
                                        color: Colors.transparent,
                                        child: Text('PixelShare',
                                            style: logoStyle)),
                                    Form(
                                        child: ListView(
                                            shrinkWrap: true,
                                            children: <Widget>[
                                          _usernameField(state),
                                          _type == 'Register'
                                              ? _emailField(state)
                                              : Container(),
                                          _passwordField(state),
                                        ])),
                                    SizedBox(
                                      height: 20,
                                    ),
                                    SubmitAnimatedButton(
                                        state: state,
                                        type: _type,
                                        submit: () => _submit(state)),
                                    _type == 'Login'
                                        ? _forgetPassButton()
                                        : Container()
                                  ],
                                ))
                              ]),
                            ),
                      ))));
        });
  }

  Widget _usernameField(RegisterState state) {
    return Padding(
      padding: EdgeInsets.only(left: 30, right: 70, top: 4, bottom: 4),
      child: Material(
        borderRadius: BorderRadius.circular(10.0),
        color: Colors.white.withOpacity(0.1),
        elevation: 0,
        child: TextFormField(
          controller: _usernameController,
          decoration: InputDecoration(
            icon: Icon(Icons.person_outline),
            labelText: 'Username',
          ),
          autovalidate: true,
          validator: (_) {
            return state.isUsernameValid || state.username.length < 1
                ? null
                : 'Invalid Username';
          },
        ),
      ),
    );
  }

  Widget _emailField(RegisterState state) {
    return Padding(
        padding: EdgeInsets.only(left: 30, right: 70, top: 4, bottom: 4),
        child: Material(
            borderRadius: BorderRadius.circular(20.0),
            color: Colors.white.withOpacity(0.1),
            elevation: 0,
            child: TextFormField(
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
            )));
  }

  Widget _passwordField(RegisterState state) {
    return Padding(
      padding: EdgeInsets.only(left: 30, right: 70, top: 4, bottom: 4),
      child: Material(
        borderRadius: BorderRadius.circular(10.0),
        color: Colors.white.withOpacity(0.1),
        elevation: 0,
        child: TextFormField(
          controller: _passwordController,
          decoration: InputDecoration(
            icon: Icon(Icons.lock_outline),
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
      ),
    );
  }

  Widget _forgetPassButton() {
    return Padding(
      padding: EdgeInsets.symmetric(horizontal: 70, vertical: 0),
      child: FlatButton(
          color: Colors.transparent,
          onPressed: () => _showRecoverPass(),
          child: Text('Forgot Password?', style: recoverButtonStyle)),
    );
  }

  void _showRecoverPass() {
    print('test recover pass');
  }

  @override
  void dispose() {
    _usernameController.dispose();
    _emailController.dispose();
    _passwordController.dispose();
    _registerBloc.dispose();
    _animationController.dispose();
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
    if (_type == 'Register') {
      _userBloc.dispatch(Registering(
          username: state.username,
          password: state.password,
          email: state.email));
    } else {
      _userBloc.dispatch(
          LoggingIn(username: state.username, password: state.password));
    }
  }
}


