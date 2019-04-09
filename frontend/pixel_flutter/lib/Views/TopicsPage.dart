import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:http/http.dart' as http;

import 'package:pixel_flutter/blocs/Blocs.dart';

import 'package:pixel_flutter/components/NavigationBar/NavBarCommon.dart';
import 'package:pixel_flutter/components/NavigationBar/TabNavBar.dart';

import 'package:pixel_flutter/components/Loader/CenterLoader.dart';
import 'package:pixel_flutter/components/Loader/BottomLoader.dart';
import 'package:pixel_flutter/Views/TopicView.dart';

class TopicsPage extends StatefulWidget {
  @override
  _TopicsPageState createState() => _TopicsPageState();
}

class _TopicsPageState extends State<TopicsPage> {
  final _scrollController = ScrollController();
  final TopicBloc _topicBloc = TopicBloc(httpClient: http.Client());
  final _scrollThreshold = 200.0;

  final _scaffoldKey = new GlobalKey<ScaffoldState>();

  _TopicsPageState() {
    _scrollController.addListener(_onScroll);
    _topicBloc.dispatch(TopicAPI());
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: _topicBloc,
        builder: (BuildContext context, TopicState state) {
          return Scaffold(
            key: _scaffoldKey,
            bottomNavigationBar: TabNavBar(1),
            endDrawer: Container(
              child: Center(child: Text('abcdefg')),
            ),
            body: CustomScrollView(
                controller: _scrollController,
                slivers: <Widget>[
                  NavBarCommon(title: 'test', isClose: false),
                  Sliverlist(state)
                ]),
          );
        });
  }

  @override
  void dispose() {
    _topicBloc.dispose();
    super.dispose();
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicBloc.dispatch(TopicAPI());
    }
  }
}

class Sliverlist extends StatelessWidget {
  final state;
  Sliverlist(this.state);

  @override
  Widget build(BuildContext context) {
    if (state is TopicError) {
      return CenterLoader();
    }
    if (state is TopicLoaded) {
      if (state.topics.isEmpty) {
        return CenterLoader();
      }
      return SliverList(
        delegate: SliverChildBuilderDelegate((context, index) {
          return index >= state.topics.length
              ? BottomLoader()
              : TopicView(state.topics[index]);
        },
            childCount: state.hasReachedMax
                ? state.topics.length
                : state.topics.length + 1),
      );
    } else {
      return CenterLoader();
    }
  }
}

