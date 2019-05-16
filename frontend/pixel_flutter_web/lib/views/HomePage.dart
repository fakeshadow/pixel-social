import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter_web/blocs/UserBlocs.dart';
import 'package:pixel_flutter_web/blocs/TopicsBlocs.dart';
import 'package:pixel_flutter_web/blocs/CategoryBlocs.dart';

import 'package:pixel_flutter_web/components/UserDrawer.dart';
import 'package:pixel_flutter_web/components/TopicsLayout.dart';
import 'package:pixel_flutter_web/components/SideMenu.dart';
import 'package:pixel_flutter_web/components/GeneralBackground.dart';
import 'package:pixel_flutter_web/components/BottomLoader.dart';
import 'package:pixel_flutter_web/components/FloatingAppBar.dart';
import 'package:pixel_flutter_web/components/TopicTile.dart';

import 'package:pixel_flutter_web/style/text.dart';
import 'package:pixel_flutter_web/style/colors.dart';


class HomePage extends StatefulWidget with env {
  HomePage({Key key, this.title}) : super(key: key);

  final String title;

  @override
  _HomePageState createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  TopicsBloc _topicsBloc;
  final _scrollController = ScrollController();
  final _scrollThreshold = 300.0;

  @override
  void initState() {
    BlocProvider.of<CategoryBloc>(context).dispatch(LoadCategories());
    _topicsBloc = TopicsBloc();
    _topicsBloc.dispatch(GetTopics(categoryId: 1));
    _scrollController.addListener(_onScroll);
    super.initState();
  }

  @override
  void dispose() {
    _topicsBloc.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return WillPopScope(
      onWillPop: onWillPop,
      child: BlocBuilder(
          bloc: BlocProvider.of<UserBloc>(context),
          builder: (context, userState) {
            return Scaffold(
              endDrawer: UserDrawer(),
              body: BlocListener(
                bloc: BlocProvider.of<ErrorBloc>(context),
                listener: (context, state) async {
                  snackbarController(context, state);
                },
                child: Stack(
                  alignment: Alignment.centerLeft,
                  children: [
                    GeneralBackground(),
                    CustomScrollView(
                      controller: _scrollController,
                      slivers: [
                        FloatingAppBar(title: 'pixelshare example'),
                        SliverPadding(
                          padding: EdgeInsets.only(
                            left: MediaQuery.of(context).size.width >
                                    widget.BREAK_POINT_WIDTH
                                ? MediaQuery.of(context).size.width * 0.2
                                : 0,
                            right: MediaQuery.of(context).size.width >
                                    widget.BREAK_POINT_WIDTH_SM
                                ? MediaQuery.of(context).size.width * 0.4
                                : 0,
                          ),
                          sliver: TopicsLayout(
                            topicsBloc: _topicsBloc,
                          ),
                        )
                      ],
                    ),
                    Padding(
                        padding: EdgeInsets.only(
                          left: MediaQuery.of(context).size.width * 0.6 + 30),
                        child: MediaQuery.of(context).size.width >
                                widget.BREAK_POINT_WIDTH_SM
                            ? SideMenu()
                            : Container())
                  ],
                ),
              ),
            );
          }),
    );
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicsBloc.dispatch(GetTopics(categoryId: 1));
    }
  }

  Future<bool> onWillPop() {
    return showDialog(
        context: context,
        builder: (context) => AlertDialog(
              title: Text('Leaving?'),
              content: Text('Do you want to exit the app'),
              actions: <Widget>[
                FlatButton(
                  onPressed: () => Navigator.pop(context, false),
                  child: Text(
                    'Cancel',
                    style: recoverButtonStyle,
                  ),
                ),
                RaisedButton(
                  color: primaryColor,
                  onPressed: () => Navigator.pop(context, true),
                  child: Text(
                    'Confirm',
                    style: submitButtonStyle.copyWith(fontSize: 16),
                  ),
                )
              ],
            ));
  }

  snackbarController(BuildContext context, ErrorState state) async {
    if (state is NoSnack) {
      Scaffold.of(context).hideCurrentSnackBar();
    } else if (state is ShowSuccess) {
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        backgroundColor: Colors.green,
        content: Text(
          state.success,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 25, fontWeight: FontWeight.bold),
        ),
      ));
    } else if (state is ShowError) {
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        backgroundColor: Colors.deepOrangeAccent,
        content: Text(
          state.error,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 25, fontWeight: FontWeight.bold),
        ),
      ));
    }
  }
}

class TopicsLayoutNew extends StatelessWidget with env {
  final TopicsBloc topicsBloc;

  TopicsLayoutNew({this.topicsBloc});

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: topicsBloc,
        builder: (context, state) {
          if (state is TopicsLoaded) {
            return SliverList(
              delegate: SliverChildBuilderDelegate((context, index) {
                return index >= state.topics.length && !state.hasReachedMax
                    ? BottomLoader()
                    : TopicTile(topic: state.topics[index]);
              },
                  childCount: state.hasReachedMax
                      ? state.topics.length
                      : state.topics.length + 1),
            );
          } else {
            return SliverToBoxAdapter(
              child: Center(
                child: SizedBox(
                  width: 30,
                  height: 30,
                  child: CircularProgressIndicator(),
                ),
              ),
            );
          }
        });
  }
}


